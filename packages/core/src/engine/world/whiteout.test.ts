import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { WhiteoutManager } from "@pokecrystal/core/engine/world/whiteout";
import {
  fade_in_from_white,
  fade_out_to_white,
  heal_party,
  warp_to_spawn_point,
} from "@pokecrystal/core/engine/world/special-events";
import { STANDARD_SCRIPT_HANDLERS } from "@pokecrystal/core/engine/world/story-events/specials/handlers";
import type { ScriptRunner } from "@pokecrystal/core/engine/world/story-events/runner";
import { createTestPokemon } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { findSpawnForMap } from "@pokecrystal/core/engine/world/maps";

jest.mock("@pokecrystal/core/engine/world/special-events", () => ({
  __esModule: true,
  fade_in_from_white: jest.fn(),
  fade_out_to_white: jest.fn(),
  heal_party: jest.fn(),
  warp_to_spawn_point: jest.fn(),
}));

jest.mock("@pokecrystal/core/engine/world/story-events/specials/handlers", () => ({
  __esModule: true,
  STANDARD_SCRIPT_HANDLERS: {
    BugContestResultsWarpScript: jest.fn(),
  },
}));

type TestOverworld = {
  dialogue?: { waiting_for_input?: boolean; pending_waits?: number } | null;
  script_runner?: ScriptRunner | null;
  input_capture_active?: boolean;
  lock_player_movement?: jest.Mock;
  unlock_player_movement?: jest.Mock;
};

const runUpdates = (manager: WhiteoutManager, frames: number): void => {
  for (let i = 0; i < frames; i += 1) {
    manager.update();
  }
};

describe("WhiteoutManager", () => {
  let game_state: ReturnType<typeof createInitialGameState>;
  let event_manager: EventManager;
  let overworld: TestOverworld;

  beforeEach(() => {
    game_state = createInitialGameState();
    event_manager = new EventManager(game_state);
    overworld = {
      dialogue: null,
      script_runner: null,
      input_capture_active: false,
      lock_player_movement: jest.fn(),
      unlock_player_movement: jest.fn(),
    };
    (fade_in_from_white as jest.Mock).mockClear();
    (fade_out_to_white as jest.Mock).mockClear();
    (heal_party as jest.Mock).mockClear();
    (warp_to_spawn_point as jest.Mock).mockClear();
    (STANDARD_SCRIPT_HANDLERS.BugContestResultsWarpScript as jest.Mock).mockClear();
  });

  it("shows the whiteout text and advances to fade when dialogue has no pending waits", () => {
    game_state.sram.player_name = "KRIS";
    overworld.input_capture_active = true;
    overworld.dialogue = { waiting_for_input: false };

    const manager = new WhiteoutManager(game_state, overworld, event_manager);
    const dispatchSpy = jest.spyOn(event_manager, "dispatch");

    event_manager.dispatch(new Event("battle_complete", { result: 1 }));

    manager.update();

    expect(overworld.lock_player_movement).toHaveBeenCalledTimes(1);
    expect(overworld.input_capture_active).toBe(false);
    expect(game_state.wram.reload_map_after_battle).toBe(false);
    expect(dispatchSpy.mock.calls.map(([event]) => event.name)).not.toContain("show_text");
    expect(fade_out_to_white).not.toHaveBeenCalled();

    manager.update();
    expect(dispatchSpy.mock.calls.map(([event]) => event.name)).toContain("show_text");
    expect(fade_out_to_white).not.toHaveBeenCalled();

    manager.update();
    expect(fade_out_to_white).toHaveBeenCalledTimes(1);
  });

  it("heals, halves money, resolves the ASM whiteout spawn, and warps after the fade delay", () => {
    game_state.sram.money = 3000;
    game_state.wram.wLastSpawnMapGroup = 23;
    game_state.wram.wLastSpawnMapNumber = 9;
    const expectedSpawn = findSpawnForMap(game_state.wram.wLastSpawnMapGroup, game_state.wram.wLastSpawnMapNumber);
    expect(expectedSpawn).toBeDefined();
    (warp_to_spawn_point as jest.Mock).mockReturnValue(true);

    const manager = new WhiteoutManager(game_state, overworld, event_manager);
    event_manager.dispatch(new Event("battle_complete", { result: 1 }));

    runUpdates(manager, 50);

    expect(heal_party).toHaveBeenCalledTimes(1);
    expect(game_state.wram.wDefaultSpawnpoint).toBe(expectedSpawn?.[0]);
    expect(warp_to_spawn_point).toHaveBeenCalledTimes(1);
    expect(game_state.sram.money).toBe(1500);
    expect(fade_in_from_white).toHaveBeenCalledTimes(1);
  });

  it("uses the saved last spawn when the WRAM whiteout spawn map is not a spawn point", () => {
    game_state.wram.wLastSpawnMapGroup = 99;
    game_state.wram.wLastSpawnMapNumber = 99;
    game_state.sram.last_spawn_map_group = 23;
    game_state.sram.last_spawn_map_number = 9;
    const expectedSpawn = findSpawnForMap(game_state.sram.last_spawn_map_group, game_state.sram.last_spawn_map_number);
    expect(expectedSpawn).toBeDefined();
    (warp_to_spawn_point as jest.Mock).mockReturnValue(true);

    const manager = new WhiteoutManager(game_state, overworld, event_manager);
    event_manager.dispatch(new Event("battle_complete", { result: 1 }));

    runUpdates(manager, 50);

    expect(game_state.wram.wDefaultSpawnpoint).toBe(expectedSpawn?.[0]);
    expect(game_state.wram.wLastSpawnMapGroup).toBe(23);
    expect(game_state.wram.wLastSpawnMapNumber).toBe(9);
    expect(warp_to_spawn_point).toHaveBeenCalledTimes(1);
  });

  it("falls back to HOME when neither WRAM nor saved spawn resolves", () => {
    game_state.wram.wLastSpawnMapGroup = 99;
    game_state.wram.wLastSpawnMapNumber = 99;
    game_state.sram.last_spawn_map_group = 88;
    game_state.sram.last_spawn_map_number = 88;
    (warp_to_spawn_point as jest.Mock).mockReturnValue(true);

    const manager = new WhiteoutManager(game_state, overworld, event_manager);
    event_manager.dispatch(new Event("battle_complete", { result: 1 }));

    runUpdates(manager, 50);

    expect(game_state.wram.wDefaultSpawnpoint).toBe(0);
    expect(game_state.wram.wLastSpawnMapGroup).toBe(24);
    expect(game_state.wram.wLastSpawnMapNumber).toBe(7);
    expect(warp_to_spawn_point).toHaveBeenCalledTimes(1);
  });

  it("uses the bug contest results warp handler without halving money", () => {
    game_state.sram.money = 1200;
    game_state.wram.engine_flags.ENGINE_BUG_CONTEST_TIMER = true;
    overworld.script_runner = {} as ScriptRunner;

    const manager = new WhiteoutManager(game_state, overworld, event_manager);
    event_manager.dispatch(new Event("battle_complete", { result: 1 }));

    runUpdates(manager, 60);

    expect(STANDARD_SCRIPT_HANDLERS.BugContestResultsWarpScript).toHaveBeenCalledTimes(1);
    expect(warp_to_spawn_point).not.toHaveBeenCalled();
    expect(game_state.sram.money).toBe(1200);
    expect(fade_in_from_white).toHaveBeenCalledTimes(1);
  });

  it("ignores whiteout when party still has usable pokemon despite loss flag", () => {
    const pokemon = createTestPokemon("TOTODILE", 10, { hp: 10, max_hp: 10 });
    game_state.sram.party.pokemon = [pokemon, null, null, null, null, null];

    const manager = new WhiteoutManager(game_state, overworld, event_manager);
    event_manager.dispatch(new Event("battle_complete", { result: 1 }));

    manager.update();

    expect(fade_out_to_white).not.toHaveBeenCalled();
    expect(game_state.wram.reload_map_after_battle).toBe(false);
  });

  it("whiteouts from overworld updates when the party is fully fainted", () => {
    const pokemon = createTestPokemon("CYNDAQUIL", 10, { hp: 0, max_hp: 22 });
    game_state.sram.party.pokemon = [pokemon, null, null, null, null, null];

    const manager = new WhiteoutManager(game_state, overworld, event_manager);

    manager.update();
    manager.update();
    manager.update();

    expect(overworld.lock_player_movement).toHaveBeenCalledTimes(1);
    expect(fade_out_to_white).toHaveBeenCalledTimes(1);
    expect(game_state.wram.reload_map_after_battle).toBe(false);
  });

  it("skips whiteout when battle type allows losing", () => {
    const pokemon = createTestPokemon("TOTODILE", 10, { hp: 0, max_hp: 10 });
    game_state.sram.party.pokemon = [pokemon, null, null, null, null, null];
    game_state.wram.battle_type = "BATTLETYPE_CANLOSE";

    const manager = new WhiteoutManager(game_state, overworld, event_manager);
    event_manager.dispatch(new Event("battle_complete", { result: 1 }));

    manager.update();

    expect(fade_out_to_white).not.toHaveBeenCalled();
    expect(overworld.lock_player_movement).not.toHaveBeenCalled();
  });
});
