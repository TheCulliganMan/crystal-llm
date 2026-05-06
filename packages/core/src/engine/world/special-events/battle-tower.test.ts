import { createInitialGameState } from "@pokecrystal/core/core/state";
import { createScriptRunnerStub, createTestPokemon } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { check_for_battle_tower_rules } from "./battle-tower";

describe("check_for_battle_tower_rules", () => {
  it("passes exactly three non-egg Pokemon with unique species and held items", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ variables: {} });
    gameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", 1, { item: "BERRY" }),
      createTestPokemon("CYNDAQUIL", 2, { item: "GOLD_BERRY" }),
      createTestPokemon("TOTODILE", 3, { item: null }),
      null,
      null,
      null,
    ];

    expect(check_for_battle_tower_rules(gameState, { runner })).toBe("FALSE");
    expect(runner.last_condition_result).toBe(false);
    expect(runner.variables.battle_tower_rule_failure).toBeNull();
  });

  it("rejects duplicate species before duplicate held items like the ASM jumptable", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ variables: {} });
    gameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", 1, { item: "BERRY" }),
      createTestPokemon("CHIKORITA", 1, { item: "BERRY" }),
      createTestPokemon("TOTODILE", 3, { item: "GOLD_BERRY" }),
      null,
      null,
      null,
    ];

    expect(check_for_battle_tower_rules(gameState, { runner })).toBe("TRUE");
    expect(runner.last_condition_result).toBe(true);
    expect(runner.variables.battle_tower_rule_failure).toBe(
      "TheMonMustAllBeDifferentKindsText"
    );
  });

  it("rejects eggs after species and item uniqueness checks", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ variables: {} });
    gameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", 1, { item: "BERRY" }),
      createTestPokemon("CYNDAQUIL", 2, { item: "BERRY" }),
      createTestPokemon("EGG", 253, { item: "BERRY" }),
      null,
      null,
      null,
    ];

    expect(check_for_battle_tower_rules(gameState, { runner })).toBe("TRUE");
    expect(runner.variables.battle_tower_rule_failure).toBe("TheMonMustNotHoldTheSameItemsText");
  });
});
