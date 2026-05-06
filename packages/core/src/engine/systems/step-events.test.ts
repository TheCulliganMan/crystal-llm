import { StatusCondition } from "@pokecrystal/core/core/enums/battle";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { StepEventSystem } from "@pokecrystal/core/engine/systems/step-events";
import { createTestPokemon } from "@pokecrystal/core/engine/world/story-events/test-utils";

describe("StepEventSystem poison steps", () => {
  it("applies poison damage to party members in SRAM every four steps", () => {
    const gameState = createInitialGameState();
    const poisoned = createTestPokemon("ODDISH", 43, {
      hp: 3,
      max_hp: 3,
      status: StatusCondition.POISON,
    });
    gameState.sram.party.pokemon = [poisoned, null, null, null, null, null];

    const system = new StepEventSystem(gameState);

    for (let i = 0; i < 3; i += 1) {
      const result = system.process_step();
      expect(result.poison_result).toBeNull();
    }

    const result = system.process_step();

    expect(result.poison_result?.damagedNames).toEqual(["ODDISH"]);
    expect(gameState.sram.party.pokemon[0]?.hp).toBe(2);
  });

  it("applies poison damage when status payload is a lowercase string", () => {
    const gameState = createInitialGameState();
    const poisoned = createTestPokemon("GRIMER", 46, {
      hp: 4,
      max_hp: 4,
      status: StatusCondition.POISON,
    });
    (poisoned as { status: unknown }).status = "poison";
    gameState.sram.party.pokemon = [poisoned, null, null, null, null, null];

    const system = new StepEventSystem(gameState);

    for (let i = 0; i < 3; i += 1) {
      expect(system.process_step().poison_result).toBeNull();
    }

    const result = system.process_step();

    expect(result.poison_result?.damagedNames).toEqual(["GRIMER"]);
    expect(gameState.sram.party.pokemon[0]?.hp).toBe(3);
  });

  it("applies poison damage when status payload is a named object", () => {
    const gameState = createInitialGameState();
    const poisoned = createTestPokemon("DROWZEE", 47, {
      hp: 4,
      max_hp: 4,
      status: StatusCondition.POISON,
    });
    (poisoned as { status: unknown }).status = { name: "poison" };
    gameState.sram.party.pokemon = [poisoned, null, null, null, null, null];

    const system = new StepEventSystem(gameState);

    for (let i = 0; i < 3; i += 1) {
      expect(system.process_step().poison_result).toBeNull();
    }

    const result = system.process_step();

    expect(result.poison_result?.damagedNames).toEqual(["DROWZEE"]);
    expect(gameState.sram.party.pokemon[0]?.hp).toBe(3);
  });
});

describe("StepEventSystem egg steps", () => {
  it("hatches an egg only when the ASM-style decrement reaches zero", () => {
    const gameState = createInitialGameState();
    gameState.wram.step_count = 0x7f;
    const egg = createTestPokemon("TOGEPI", 175, {
      nickname: "EGG",
      happiness: 1,
      hp: 8,
      max_hp: 8,
    });
    gameState.sram.party.pokemon = [egg, null, null, null, null, null];

    const system = new StepEventSystem(gameState);
    const result = system.process_step();

    expect(result.egg_hatched).toBe(true);
    expect(result.hatched_species).toBe("TOGEPI");
    expect(egg.nickname).toBe("TOGEPI");
    expect(egg.happiness).toBe(0x78);
  });

  it("wraps the egg hatch counter from 0 to 255 like DEC [hl] in ASM", () => {
    const gameState = createInitialGameState();
    gameState.wram.step_count = 0x7f;
    const egg = createTestPokemon("TOGEPI", 175, {
      nickname: "EGG",
      happiness: 0,
    });
    gameState.sram.party.pokemon = [egg, null, null, null, null, null];

    const system = new StepEventSystem(gameState);
    const result = system.process_step();

    expect(result.egg_hatched).toBe(false);
    expect(result.hatched_species).toBeNull();
    expect(egg.nickname).toBe("EGG");
    expect(egg.happiness).toBe(0xff);
  });

  it("stops decrementing later eggs once the first hatch-ready egg is found", () => {
    const gameState = createInitialGameState();
    gameState.wram.step_count = 0x7f;
    const firstEgg = createTestPokemon("TOGEPI", 175, {
      nickname: "EGG",
      happiness: 1,
    });
    const secondEgg = createTestPokemon("PICHU", 172, {
      nickname: "EGG",
      happiness: 2,
    });
    gameState.sram.party.pokemon = [firstEgg, secondEgg, null, null, null, null];

    const system = new StepEventSystem(gameState);
    const result = system.process_step();

    expect(result.egg_hatched).toBe(true);
    expect(firstEgg.nickname).toBe("TOGEPI");
    expect(secondEgg.happiness).toBe(2);
  });

  it("matches CountStep ordering by skipping daycare and poison when egg hatch event triggers", () => {
    const gameState = createInitialGameState();
    gameState.wram.step_count = 0x7f;
    gameState.wram.poison_step_count = 3;
    const egg = createTestPokemon("TOGEPI", 175, {
      nickname: "EGG",
      happiness: 1,
    });
    const poisoned = createTestPokemon("ODDISH", 43, {
      hp: 3,
      max_hp: 3,
      status: StatusCondition.POISON,
    });
    const dayCare = { advance_steps: jest.fn() };
    gameState.sram.party.pokemon = [egg, poisoned, null, null, null, null];

    const system = new StepEventSystem(gameState, { day_care: dayCare });
    const result = system.process_step();

    expect(result.egg_hatched).toBe(true);
    expect(result.poison_result).toBeNull();
    expect(poisoned.hp).toBe(3);
    expect(gameState.wram.poison_step_count).toBe(4);
    expect(dayCare.advance_steps).not.toHaveBeenCalled();
  });
});
