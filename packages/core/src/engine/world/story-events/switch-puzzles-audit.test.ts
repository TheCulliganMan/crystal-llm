import path from "path";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader, type ScriptEntry } from "@pokecrystal/core/core/data-loader";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { readJsonAssetSync } from "@pokecrystal/core/core/asset-reader";
import { getDataDir } from "@pokecrystal/core/core/paths";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { SPECIAL_FUNCTIONS } from "@pokecrystal/core/engine/world/special-events/registry";
import { ScriptRunnerImpl } from "./runner";
import { createTestPokemon } from "./test-utils";

type ScriptMap = Record<string, ScriptEntry[]>;

const drainRunner = (runner: ScriptRunnerImpl, limit = 12): void => {
  for (let i = 0; i < limit && runner.is_busy; i += 1) {
    runner.resume();
  }
};

const loadMapScripts = (mapName: string): ScriptMap => {
  const mapPath = path.join(getDataDir(), "maps", `${mapName}.json`);
  return readJsonAssetSync<ScriptMap>(mapPath);
};

const GOLDENROD_SWITCH_PARENT = "GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors";
const GOLDENROD_DOOR_PARTS: Record<number, Array<readonly [number, number, number, number]>> = {
  1: [[16, 6, 0x3e, 0x2d]],
  2: [[10, 6, 0x3e, 0x2d]],
  3: [[2, 6, 0x3e, 0x2d]],
  4: [[2, 10, 0x3e, 0x2d]],
  5: [[10, 10, 0x3e, 0x2d]],
  6: [[16, 10, 0x3e, 0x2d]],
  7: [[12, 6, 0x3f, 0x2a], [12, 8, 0x3d, 0x2d]],
  8: [[6, 6, 0x3f, 0x2a], [6, 8, 0x3d, 0x2d]],
  9: [[12, 10, 0x3f, 0x2a], [12, 12, 0x3d, 0x2d]],
  10: [[6, 10, 0x3f, 0x2a], [6, 12, 0x3d, 0x2d]],
  11: [[18, 10, 0x3f, 0x2a], [18, 12, 0x3d, 0x2d]],
};

const doorWriteCalls = (
  doorIds: readonly number[],
  state: "open" | "closed"
): Array<[number, number, number]> =>
  doorIds.flatMap((doorId) =>
    (GOLDENROD_DOOR_PARTS[doorId] ?? []).map(([x, y, closed, open]) => [
      x / 2,
      y / 2,
      state === "open" ? open : closed,
    ])
  );

const goldenrodDoorLayouts: Array<{
  readonly position: number;
  readonly open: readonly number[];
  readonly closed: readonly number[];
  readonly actions: ReadonlyArray<readonly ["open" | "closed", number]>;
  readonly finalPosition: number;
}> = [
  {
    position: 0,
    open: [],
    closed: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
    actions: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11].map((doorId) => ["closed", doorId]),
    finalPosition: 0,
  },
  {
    position: 1,
    open: [1, 7, 10],
    closed: [6, 8, 9, 11],
    actions: [["open", 1], ["open", 7], ["open", 10], ["closed", 6], ["closed", 8], ["closed", 9], ["closed", 11]],
    finalPosition: 1,
  },
  {
    position: 2,
    open: [2, 8, 9],
    closed: [5, 7, 10, 11],
    actions: [["open", 2], ["open", 8], ["open", 9], ["closed", 5], ["closed", 7], ["closed", 10], ["closed", 11]],
    finalPosition: 2,
  },
  {
    position: 3,
    open: [3, 7, 10],
    closed: [4, 8, 9, 11],
    actions: [["open", 3], ["open", 7], ["open", 10], ["closed", 4], ["closed", 8], ["closed", 9], ["closed", 11]],
    finalPosition: 3,
  },
  {
    position: 4,
    open: [4, 8, 9],
    closed: [3, 7, 10, 11],
    actions: [["open", 4], ["open", 8], ["open", 9], ["closed", 3], ["closed", 7], ["closed", 10], ["closed", 11]],
    finalPosition: 4,
  },
  {
    position: 5,
    open: [5, 7, 10],
    closed: [2, 8, 9, 11],
    actions: [["open", 5], ["open", 7], ["open", 10], ["closed", 2], ["closed", 8], ["closed", 9], ["closed", 11]],
    finalPosition: 5,
  },
  {
    position: 6,
    open: [6, 8, 9, 11],
    closed: [1, 7, 10],
    actions: [["open", 6], ["open", 8], ["open", 9], ["open", 11], ["closed", 1], ["closed", 7], ["closed", 10]],
    finalPosition: 6,
  },
  {
    position: 7,
    open: [3, 5, 6, 8, 9, 11],
    closed: [1, 2, 4, 7, 10],
    actions: [
      ["closed", 1],
      ["closed", 2],
      ["open", 3],
      ["closed", 4],
      ["open", 5],
      ["open", 6],
      ["closed", 7],
      ["open", 8],
      ["open", 9],
      ["closed", 10],
      ["open", 11],
    ],
    finalPosition: 6,
  },
];

const createRunner = (scripts: ScriptMap, overworldOverrides: Partial<OverworldEngine> = {}): ScriptRunnerImpl => {
  const gameState = createInitialGameState();
  const eventManager = new EventManager(gameState);
  const dataLoader = new DataLoader();
  dataLoader.get_script = (name: string, parent?: string) => {
    if (parent && name.startsWith(".")) {
      return scripts[`${name}@${parent}`] ?? scripts[name] ?? null;
    }
    return scripts[name] ?? null;
  };
  dataLoader.get_text = (label: string) => label;
  const overworld = {
    current_map_name: "AUDIT_MAP",
    dialogue: { active: true, acknowledge_wait: () => true },
    _write_metatile: () => undefined,
    _refresh_warp_permissions: () => undefined,
    wait_sfx: () => undefined,
    ...overworldOverrides,
  } as unknown as OverworldEngine;
  return new ScriptRunnerImpl(gameState, eventManager, dataLoader, overworld);
};

describe("switch puzzle audit", () => {
  it("toggles Team Rocket B1F security cameras exactly once", () => {
    const scripts = loadMapScripts("TeamRocketBaseB1F");
    const runner = createRunner(scripts);

    runner.run("TeamRocketBaseB1FSecretSwitch");
    drainRunner(runner);

    expect(runner.gameState.wram.event_flags.EVENT_TURNED_OFF_SECURITY_CAMERAS).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SECURITY_CAMERA_1).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SECURITY_CAMERA_2).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SECURITY_CAMERA_3).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SECURITY_CAMERA_4).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SECURITY_CAMERA_5).toBe(true);

    runner.run("TeamRocketBaseB1FSecretSwitch");
    drainRunner(runner);

    expect(runner.gameState.wram.event_flags.EVENT_TURNED_OFF_SECURITY_CAMERAS).toBe(true);
  });

  it("opens Team Rocket B2F locked door when the password flag is set", () => {
    const scripts = loadMapScripts("TeamRocketBaseB2F");
    const writeMetatile = jest.fn();
    const refreshWarpPermissions = jest.fn();
    const runner = createRunner(scripts, {
      _write_metatile: writeMetatile,
      _refresh_warp_permissions: refreshWarpPermissions,
    } as unknown as Partial<OverworldEngine>);

    runner.run("TeamRocketBaseB2FLockedDoor");
    drainRunner(runner);
    expect(runner.gameState.wram.event_flags.EVENT_OPENED_DOOR_TO_ROCKET_HIDEOUT_TRANSMITTER).not.toBe(true);
    expect(writeMetatile).not.toHaveBeenCalled();

    runner.gameState.wram.event_flags.EVENT_LEARNED_HAIL_GIOVANNI = true;
    runner.run("TeamRocketBaseB2FLockedDoor");
    drainRunner(runner);

    expect(runner.gameState.wram.event_flags.EVENT_OPENED_DOOR_TO_ROCKET_HIDEOUT_TRANSMITTER).toBe(true);
    expect(writeMetatile).toHaveBeenCalledWith(7, 6, 0x07);
    expect(refreshWarpPermissions).toHaveBeenCalledTimes(1);
  });

  it("clears a defeated Team Rocket B2F Electrode instead of taking the loss reload branch", () => {
    const scripts = loadMapScripts("TeamRocketBaseB2F");
    const objectFlags: Record<string, string> = {
      TEAMROCKETBASEB2F_ELECTRODE1: "EVENT_TEAM_ROCKET_BASE_B2F_ELECTRODE_1",
      TEAMROCKETBASEB2F_ELECTRODE4: "EVENT_TEAM_ROCKET_BASE_B2F_ELECTRODE_1",
    };
    let runner: ScriptRunnerImpl;
    const removeObject = jest.fn((objectId: string | number) => {
      const flag = objectFlags[String(objectId).toUpperCase()];
      if (flag) {
        runner.gameState.wram.event_flags[flag] = true;
      }
    });
    runner = createRunner(scripts, {
      current_map_name: "TeamRocketBaseB2F",
      remove_object: removeObject,
    } as unknown as Partial<OverworldEngine>);
    runner.gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", 158, { hp: 10, max_hp: 10 }),
      null,
      null,
      null,
      null,
      null,
    ];

    runner.run("RocketElectrode1");
    runner.event_manager.dispatch(new Event("battle_complete", { result: 0 }));
    drainRunner(runner);

    expect(removeObject).toHaveBeenCalledWith("TEAMROCKETBASEB2F_ELECTRODE1");
    expect(removeObject).toHaveBeenCalledWith("TEAMROCKETBASEB2F_ELECTRODE4");
    expect(runner.gameState.wram.event_flags.EVENT_TEAM_ROCKET_BASE_B2F_ELECTRODE_1).toBe(true);
  });

  it("captures Goldenrod emergency switch regression on second toggle", () => {
    const scripts = loadMapScripts("GoldenrodUndergroundSwitchRoomEntrances");
    const runner = createRunner(scripts);
    runner._consume_script_choice = (key: string) => (key === "_yesorno_choice" ? true : null);

    runner.run("Switch1Script");
    drainRunner(runner);
    runner.run("Switch2Script");
    drainRunner(runner);
    runner.run("Switch3Script");
    drainRunner(runner);

    expect(runner.gameState.wram.event_flags.EVENT_SWITCH_1).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SWITCH_2).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SWITCH_3).toBe(true);
    expect(runner.gameState.wram.script_memory?.wUndergroundSwitchPositions).toBe(6);

    runner.run("EmergencySwitchScript");
    drainRunner(runner);

    expect(runner.gameState.wram.event_flags.EVENT_EMERGENCY_SWITCH).toBe(true);
    expect(runner.gameState.wram.script_memory?.wUndergroundSwitchPositions).toBe(6);

    runner.run("EmergencySwitchScript");
    drainRunner(runner);

    expect(runner.gameState.wram.event_flags.EVENT_EMERGENCY_SWITCH).not.toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SWITCH_1).not.toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SWITCH_2).not.toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_SWITCH_3).not.toBe(true);
    expect(runner.gameState.wram.script_memory?.wUndergroundSwitchPositions).toBe(0);
  });

  it("audits the Tin Tower Suicune battle gate and battle payload", () => {
    const scripts = loadMapScripts("TinTower1F");

    expect(scripts["TinTower1F_MapScripts"]).toContainEqual({
      command: "scene_script",
      args: ["TinTower1FSuicuneBattleScene", "SCENE_TINTOWER1F_SUICUNE_BATTLE"],
    });
    expect(scripts["TinTower1F_MapScripts"]).toContainEqual({
      command: "callback",
      args: ["MAPCALLBACK_OBJECTS", "TinTower1FNPCsCallback"],
    });
    expect(scripts["TinTower1FNPCsCallback"]).toEqual(
      expect.arrayContaining([
        { command: "checkevent", args: ["EVENT_BEAT_ELITE_FOUR"] },
        { command: "special", args: ["BeastsCheck"] },
        { command: "appear", args: ["TINTOWER1F_SUICUNE"] },
        { command: "checkevent", args: ["EVENT_FOUGHT_SUICUNE"] },
        { command: "disappear", args: ["TINTOWER1F_SUICUNE"] },
      ]),
    );
    expect(scripts["TinTower1FSuicuneBattleScript"]).toEqual(
      expect.arrayContaining([
        { command: "applymovement", args: ["TINTOWER1F_SUICUNE", "TinTower1FSuicuneApproachesMovement"] },
        { command: "cry", args: ["SUICUNE"] },
        { command: "loadwildmon", args: ["SUICUNE", "40"] },
        { command: "loadvar", args: ["VAR_BATTLETYPE", "BATTLETYPE_SUICUNE"] },
        { command: "startbattle", args: [] },
        { command: "setevent", args: ["EVENT_FOUGHT_SUICUNE"] },
        { command: "setevent", args: ["EVENT_SAW_SUICUNE_ON_ROUTE_42"] },
        { command: "setmapscene", args: ["ROUTE_42", "SCENE_ROUTE42_NOOP"] },
        { command: "setevent", args: ["EVENT_SAW_SUICUNE_ON_ROUTE_36"] },
        { command: "setmapscene", args: ["ROUTE_36", "SCENE_ROUTE36_NOOP"] },
        { command: "setevent", args: ["EVENT_SAW_SUICUNE_AT_CIANWOOD_CITY"] },
        { command: "setmapscene", args: ["CIANWOOD_CITY", "SCENE_CIANWOODCITY_NOOP"] },
        { command: "setscene", args: ["SCENE_TINTOWER1F_NOOP"] },
        { command: "reloadmapafterbattle", args: [] },
      ]),
    );
  });

  it("audits the Tin Tower Rainbow Wing stairway and Ho-Oh encounter payload", () => {
    const firstFloorScripts = loadMapScripts("TinTower1F");
    const roofScripts = loadMapScripts("TinTowerRoof");

    expect(firstFloorScripts["TinTower1F_MapScripts"]).toContainEqual({
      command: "callback",
      args: ["MAPCALLBACK_TILES", "TinTower1FStairsCallback"],
    });
    expect(firstFloorScripts["TinTower1FStairsCallback"]).toEqual(
      expect.arrayContaining([
        { command: "checkevent", args: ["EVENT_GOT_RAINBOW_WING"] },
        { command: "iftrue", args: [".DontHideStairs"] },
        { command: "changeblock", args: ["10", "2", "$09"] },
      ]),
    );
    expect(firstFloorScripts[".DontHideStairs@TinTower1FStairsCallback"]).toEqual([
      { command: "endcallback", args: [] },
    ]);
    expect(firstFloorScripts["TinTower1FNPCsCallback"]).toEqual(
      expect.arrayContaining([
        { command: "checkevent", args: ["EVENT_GOT_RAINBOW_WING"] },
        { command: "iftrue", args: [".GotRainbowWing"] },
        { command: "checkevent", args: ["EVENT_BEAT_ELITE_FOUR"] },
        { command: "special", args: ["BeastsCheck"] },
        { command: "clearevent", args: ["EVENT_TIN_TOWER_1F_WISE_TRIO_2"] },
        { command: "setevent", args: ["EVENT_TIN_TOWER_1F_WISE_TRIO_1"] },
      ]),
    );
    expect(roofScripts["TinTowerRoofHoOhCallback"]).toEqual(
      expect.arrayContaining([
        { command: "checkevent", args: ["EVENT_FOUGHT_HO_OH"] },
        { command: "checkitem", args: ["RAINBOW_WING"] },
        { command: "appear", args: ["TINTOWERROOF_HO_OH"] },
        { command: "disappear", args: ["TINTOWERROOF_HO_OH"] },
      ]),
    );
    expect(roofScripts["TinTowerHoOh"]).toEqual(
      expect.arrayContaining([
        { command: "setevent", args: ["EVENT_FOUGHT_HO_OH"] },
        { command: "loadvar", args: ["VAR_BATTLETYPE", "BATTLETYPE_FORCEITEM"] },
        { command: "loadwildmon", args: ["HO_OH", "60"] },
        { command: "startbattle", args: [] },
        { command: "reloadmapafterbattle", args: [] },
        { command: "setevent", args: ["EVENT_SET_WHEN_FOUGHT_HO_OH"] },
      ]),
    );
  });

  it("audits Burned Tower beast release setup for the later Suicune path", () => {
    const scripts = loadMapScripts("BurnedTowerB1F");

    expect(scripts["BurnedTowerB1F_MapEvents"]).toContainEqual({
      command: "coord_event",
      args: ["10", "6", "SCENE_BURNEDTOWERB1F_RELEASE_THE_BEASTS", "ReleaseTheBeasts"],
    });
    expect(scripts["ReleaseTheBeasts"]).toEqual(
      expect.arrayContaining([
        { command: "appear", args: ["BURNEDTOWERB1F_SUICUNE1"] },
        { command: "cry", args: ["SUICUNE"] },
        { command: "applymovement", args: ["BURNEDTOWERB1F_SUICUNE1", "BurnedTowerSuicuneMovement1"] },
        { command: "applymovement", args: ["BURNEDTOWERB1F_SUICUNE1", "BurnedTowerSuicuneMovement2"] },
        { command: "applymovement", args: ["BURNEDTOWERB1F_SUICUNE1", "BurnedTowerSuicuneMovement3"] },
        { command: "setevent", args: ["EVENT_RELEASED_THE_BEASTS"] },
        { command: "special", args: ["InitRoamMons"] },
        { command: "setmapscene", args: ["CIANWOOD_CITY", "SCENE_CIANWOODCITY_SUICUNE_AND_EUSINE"] },
        { command: "clearevent", args: ["EVENT_SAW_SUICUNE_AT_CIANWOOD_CITY"] },
      ]),
    );
  });

  it("continues the solved Kabuto puzzle script through block opening and warp check", () => {
    const scripts = loadMapScripts("RuinsOfAlphKabutoChamber");
    const writeMetatile = jest.fn();
    const refreshWarpPermissions = jest.fn();
    const checkForWarpEvent = jest.fn(() => true);
    const showEmote = jest.fn();
    const startEarthquake = jest.fn();
    const waitSfx = jest.fn((callback?: () => void) => callback?.());
    const playerObject = { x: 7, y: 7, applyMovement: jest.fn() };
    const queueMovementTask = jest.fn((_obj: unknown, _movement: unknown[], options?: { onComplete?: () => void }) => {
      options?.onComplete?.();
    });
    const originalUnownPuzzle = SPECIAL_FUNCTIONS.UnownPuzzle;
    SPECIAL_FUNCTIONS.UnownPuzzle = (({ game_state, runner }: { game_state: unknown; runner?: ScriptRunnerImpl }) => {
      void game_state;
      if (runner) {
        runner.last_condition_result = true;
      }
      return true;
    }) as typeof originalUnownPuzzle;

    try {
      const runner = createRunner(scripts, {
        current_map_name: "RuinsOfAlphKabutoChamber",
        _write_metatile: writeMetatile,
        _refresh_warp_permissions: refreshWarpPermissions,
        check_for_warp_event: checkForWarpEvent,
        show_emote: showEmote,
        start_earthquake: startEarthquake,
        wait_sfx: waitSfx,
        get_object_by_id: jest.fn((objectId: string | number) => (objectId === "PLAYER" || objectId === 0 ? playerObject : null)),
        get_movement_data: jest.fn(() => ["step_sleep_8", "step_end"]),
        queue_movement_task: queueMovementTask,
      } as unknown as Partial<OverworldEngine>);
      runner.gameState.wram.instant_mode = true;

      runner.run("RuinsOfAlphKabutoChamberPuzzle");
      drainRunner(runner, 120);

      expect(runner.is_busy).toBe(false);
      expect(runner.gameState.wram.event_flags.EVENT_RUINS_OF_ALPH_INNER_CHAMBER_TOURISTS).toBe(true);
      expect(runner.gameState.wram.event_flags.EVENT_SOLVED_KABUTO_PUZZLE).toBe(true);
      expect(runner.gameState.wram.event_flags.EVENT_RUINS_OF_ALPH_KABUTO_CHAMBER_RECEPTIONIST).toBe(true);
      expect(runner.gameState.wram.engine_flags.ENGINE_UNLOCKED_UNOWNS_A_TO_K).toBe(true);
      expect(runner.gameState.wram.map_scenes.RuinsOfAlphInnerChamber).toBe("SCENE_RUINSOFALPHINNERCHAMBER_STRANGE_PRESENCE");
      expect(writeMetatile).toHaveBeenCalledWith(1, 1, 0x18);
      expect(writeMetatile).toHaveBeenCalledWith(2, 1, 0x19);
      expect(refreshWarpPermissions).toHaveBeenCalledTimes(1);
      expect(startEarthquake).toHaveBeenCalledWith(1, 1);
      expect(startEarthquake).toHaveBeenCalledWith(2, 1);
      expect(showEmote).toHaveBeenCalledWith("EMOTE_SHOCK", playerObject, 1);
      expect(queueMovementTask).toHaveBeenCalledWith(playerObject, ["step_sleep_8", "step_end"], expect.any(Object));
      expect(waitSfx).not.toHaveBeenCalled();
      expect(checkForWarpEvent).toHaveBeenCalledWith({ allow_script: true, ignore_cooldown: true });
      expect(runner.gameState.wram.wEnabledPlayerEvents).toBe(0xff);
    } finally {
      SPECIAL_FUNCTIONS.UnownPuzzle = originalUnownPuzzle;
    }
  });

  it("updates Goldenrod switch room door blocks when switch 1 is toggled on", () => {
    const scripts = loadMapScripts("GoldenrodUndergroundSwitchRoomEntrances");
    const writeMetatile = jest.fn();
    const refreshWarpPermissions = jest.fn();
    const runner = createRunner(scripts, {
      current_map_name: "GoldenrodUndergroundSwitchRoomEntrances",
      _write_metatile: writeMetatile,
      _refresh_warp_permissions: refreshWarpPermissions,
    } as unknown as Partial<OverworldEngine>);
    runner._consume_script_choice = (key: string) => (key === "_yesorno_choice" ? true : null);

    expect(scripts[".OpenDoor1@GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors"]).toEqual([
      { command: "changeblock", args: ["16", "6", "$2d"] },
      { command: "setevent", args: ["EVENT_DOOR_1_OPEN"] },
      { command: "end", args: [] },
    ]);

    runner.run("Switch1Script");
    drainRunner(runner);

    expect(writeMetatile).toHaveBeenCalledWith(8, 3, 0x2d);
    expect(writeMetatile).toHaveBeenCalledWith(6, 3, 0x2a);
    expect(writeMetatile).toHaveBeenCalledWith(6, 4, 0x2d);
    expect(writeMetatile).toHaveBeenCalledWith(8, 5, 0x3e);
    expect(writeMetatile).toHaveBeenCalledWith(3, 3, 0x3f);
    expect(writeMetatile).toHaveBeenCalledWith(3, 4, 0x3d);
    expect(refreshWarpPermissions).toHaveBeenCalledTimes(1);
    expect(runner.gameState.wram.event_flags.EVENT_DOOR_1_OPEN).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_DOOR_7_OPEN).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_DOOR_10_OPEN).toBe(true);
    expect(runner.gameState.wram.event_flags.EVENT_DOOR_6_OPEN).not.toBe(true);
    expect(runner.gameState.wram.script_memory?.wUndergroundSwitchPositions).toBe(1);
  });

  it.each(goldenrodDoorLayouts)(
    "applies Goldenrod switch room door layout position $position",
    ({ position, open, closed, actions, finalPosition }) => {
      const scripts = loadMapScripts("GoldenrodUndergroundSwitchRoomEntrances");
      const writeMetatile = jest.fn();
      const refreshWarpPermissions = jest.fn();
      const runner = createRunner(scripts, {
        current_map_name: "GoldenrodUndergroundSwitchRoomEntrances",
        _write_metatile: writeMetatile,
        _refresh_warp_permissions: refreshWarpPermissions,
      } as unknown as Partial<OverworldEngine>);
      runner.gameState.wram.script_memory.wUndergroundSwitchPositions = position;

      runner.run(GOLDENROD_SWITCH_PARENT);
      drainRunner(runner, 80);

      expect(writeMetatile.mock.calls).toEqual(
        actions.flatMap(([state, doorId]) => doorWriteCalls([doorId], state))
      );
      expect(refreshWarpPermissions).toHaveBeenCalledTimes(1);
      for (const doorId of open) {
        expect(runner.gameState.wram.event_flags[`EVENT_DOOR_${doorId}_OPEN`]).toBe(true);
      }
      for (const doorId of closed) {
        expect(runner.gameState.wram.event_flags[`EVENT_DOOR_${doorId}_OPEN`]).not.toBe(true);
      }
      expect(runner.gameState.wram.script_memory.wUndergroundSwitchPositions).toBe(finalPosition);
    }
  );

  it("finds only one unsupported command token in executable switch puzzle scripts", () => {
    const goldenrod = loadMapScripts("GoldenrodUndergroundSwitchRoomEntrances");
    const b1f = loadMapScripts("TeamRocketBaseB1F");
    const b2f = loadMapScripts("TeamRocketBaseB2F");
    const allScripts = { ...goldenrod, ...b1f, ...b2f };
    const runner = createRunner(allScripts);

    const scriptNames = [
      "Switch1Script",
      "Switch2Script",
      "Switch3Script",
      "EmergencySwitchScript",
      "GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors",
      ".OpenDoor1@GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors",
      ".CloseDoor1@GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors",
      "TeamRocketBaseB1FSecretSwitch",
      "TeamRocketBaseB2FTransmitterDoorCallback",
      "TeamRocketBaseB2FLockedDoor",
      ...Object.keys(goldenrod).filter((name) => name.includes("@GoldenrodUndergroundSwitchRoomEntrances_UpdateDoors")),
    ];

    const unsupported = new Set<string>();
    for (const name of scriptNames) {
      const entries = allScripts[name] ?? [];
      for (const entry of entries) {
        const command = String(entry.command ?? "").trim().toLowerCase();
        if (!command) {
          continue;
        }
        if (!runner.commandFactory.commandMap.has(command)) {
          unsupported.add(command);
        }
      }
    }

    expect(Array.from(unsupported)).toEqual([]);
  });
});
