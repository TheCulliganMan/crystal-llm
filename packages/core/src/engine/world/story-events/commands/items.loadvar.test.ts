import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { ScriptRunner } from "../runner";
import { LoadVarCommand } from "./items";

describe("LoadVarCommand", () => {
  it("strips trailing commas for battle type assignments", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = { variables: {} } as ScriptRunner;
    const command = new LoadVarCommand("VAR_BATTLETYPE,", "BATTLETYPE_CANLOSE,");
    command.runner = runner;

    command.execute(gameState, eventManager, {} as any);

    expect(gameState.wram.battle_type).toBe("BATTLETYPE_CANLOSE");
    expect(runner.variables.VAR_BATTLETYPE).toBe("BATTLETYPE_CANLOSE");
  });
});
