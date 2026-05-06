import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import {
  BUGCONTEST_DECISION_PENDING,
  BUGCONTEST_NO_CATCH,
  bug_contest_set_caught_contest_mon,
  check_party_full_after_contest,
  contest_drop_off_mons,
  give_park_balls,
} from "@pokecrystal/core/engine/world/special-events/bug-contest";
import {
  createScriptRunnerStub,
  createTestPokemon,
} from "@pokecrystal/core/engine/world/story-events/test-utils";

describe("bug contest special context", () => {
  it("uses runner and event manager from overworld context when direct context is omitted", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({
      game_state: gameState,
      event_manager: eventManager,
      variables: {},
    });
    const overworld = { script_runner: runner, event_manager: eventManager };

    const result = give_park_balls(gameState, { overworld });

    expect(result).toBe(20);
    expect(runner.last_value).toBe(20);
    expect(runner.variables._value).toBe(20);
    expect(runner.last_condition_result).toBe(true);
    expect(gameState.wram.bug_contest_state.timer_active).toBe(true);
    expect(gameState.wram.engine_flags.ENGINE_BUG_CONTEST_TIMER).toBe(true);
  });

  it("updates runner result through overworld context when the lead cannot enter", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({
      game_state: gameState,
      event_manager: eventManager,
      variables: {},
    });
    gameState.sram.party.pokemon = [
      createTestPokemon("CYNDAQUIL", 155, { hp: 0, max_hp: 20 }),
      null,
      null,
      null,
      null,
      null,
    ];

    const result = contest_drop_off_mons(gameState, {
      overworld: { script_runner: runner, event_manager: eventManager },
    });

    expect(result).toBe(1);
    expect(runner.variables._value).toBe(1);
    expect(runner.last_condition_result).toBe(false);
    expect(gameState.sram.party.pokemon[0]?.species.id).toBe("CYNDAQUIL");
  });

  it("prompts before replacing an already caught contest mon and applies the callback choice", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({
      game_state: gameState,
      event_manager: eventManager,
      variables: {},
    });
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const first = createTestPokemon("CATERPIE", 10, { level: 12, hp: 8, max_hp: 20 });
    const second = createTestPokemon("SCYTHER", 123, { level: 14, hp: 31, max_hp: 44 });

    bug_contest_set_caught_contest_mon(gameState, {
      runner,
      event_manager: eventManager,
      caught_mon: first,
    });

    const pending = bug_contest_set_caught_contest_mon(gameState, {
      runner,
      event_manager: eventManager,
      caught_mon: second,
    });

    expect(pending).toBe(BUGCONTEST_DECISION_PENDING);
    expect(gameState.wram.bug_contest_caught_mon?.species.id).toBe("CATERPIE");
    expect(gameState.wram.bug_contest_state.pending_caught_mon?.species.id).toBe("SCYTHER");
    const prompt = dispatchSpy.mock.calls
      .map(([event]) => event)
      .find((event) => event.name === "prompt_yes_no");
    expect(prompt?.data?.text).toBe("Switch to SCYTHER?");

    prompt?.data?.callback(true);

    expect(gameState.wram.bug_contest_caught_mon?.species.id).toBe("SCYTHER");
    expect(gameState.wram.bug_contest_state.caught_species).toBe("SCYTHER");
    expect(gameState.wram.bug_contest_state.caught_level).toBe(14);
    expect(gameState.wram.bug_contest_state.pending_caught_mon).toBeUndefined();
    expect(runner.last_value).toBe(0);
    expect(runner.last_condition_result).toBe(true);
  });

  it("keeps the existing contest mon when the replacement prompt is declined", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const first = createTestPokemon("WEEDLE", 13, { level: 10 });
    const second = createTestPokemon("PINSIR", 127, { level: 14 });
    bug_contest_set_caught_contest_mon(gameState, {
      event_manager: eventManager,
      caught_mon: first,
    });
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");

    bug_contest_set_caught_contest_mon(gameState, {
      event_manager: eventManager,
      caught_mon: second,
    });

    const prompt = dispatchSpy.mock.calls
      .map(([event]) => event)
      .find((event) => event.name === "prompt_yes_no");
    prompt?.data?.callback(false);

    expect(gameState.wram.bug_contest_caught_mon?.species.id).toBe("WEEDLE");
    expect(gameState.wram.bug_contest_state.pending_caught_mon).toBeUndefined();
  });

  it("clears stale pending contest catches when no mon is selected after judging", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({
      game_state: gameState,
      event_manager: eventManager,
      variables: {},
    });
    gameState.wram.bug_contest_state.pending_caught_mon = createTestPokemon("PARAS", 46);

    const result = check_party_full_after_contest(gameState, {
      overworld: { script_runner: runner, event_manager: eventManager },
    });

    expect(result).toBe(BUGCONTEST_NO_CATCH);
    expect(gameState.wram.bug_contest_state.pending_caught_mon).toBeUndefined();
    expect(runner.last_value).toBe(BUGCONTEST_NO_CATCH);
    expect(runner.last_condition_result).toBe(true);
  });
});
