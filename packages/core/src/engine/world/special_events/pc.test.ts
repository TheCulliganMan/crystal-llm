import * as modernPc from "../special-events/pc";
import { pokemon_center_pc as legacyPokemonCenterPc, BillPC as legacyBillPC } from "./pc";
import { createInitialGameState } from "@pokecrystal/core/core/state";

describe("legacy special-events/pc shim", () => {
  it("delegates pokemon_center_pc to the modern implementation", async () => {
    const gameState = createInitialGameState();
    const runner = {};
    const overworld = { ui: {} };
    const event_manager = {};
    const spy = jest.spyOn(modernPc, "pokemon_center_pc").mockResolvedValue({
      selection_name: "TURN OFF",
    });

    await legacyPokemonCenterPc(gameState, runner as any, overworld as any, event_manager as any);

    expect(spy).toHaveBeenCalledWith(gameState, {
      runner,
      overworld,
      event_manager,
    });
  });

  it("delegates BillPC to the modern implementation", async () => {
    const gameState = createInitialGameState();
    const runner = { id: "script-runner" };
    const event_manager = {};
    const overworld = {
      script_runner: runner,
      event_manager,
    };
    const spy = jest.spyOn(modernPc, "BillPC").mockResolvedValue({
      selection_name: "TURN OFF",
    });

    await legacyBillPC(gameState, overworld as any);

    expect(spy).toHaveBeenCalledWith(gameState, {
      overworld,
    });
  });
});
