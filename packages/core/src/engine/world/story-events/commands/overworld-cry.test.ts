import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { CryCommand } from "./overworld";

describe("CryCommand", () => {
  it("binds pokemon cry lookups to the data loader instance", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const audioEngine = {
      playSound: jest.fn(),
      play_sound: jest.fn(),
    };
    const dataLoader = {
      cryMap: new Map([["TOTODILE", { cry_id: "CRY_CUSTOM" }]]),
      getPokemonCry(name: string) {
        if (this !== dataLoader) {
          throw new Error("unbound cry lookup");
        }
        return this.cryMap.get(name) ?? null;
      },
    };
    const overworld = {
      data_loader: dataLoader,
      audio_engine: audioEngine,
    } as unknown as OverworldEngine;

    const command = new CryCommand("TOTODILE");
    command.execute(gameState, eventManager, overworld);

    expect(audioEngine.playSound).toHaveBeenCalledWith("CRY_CUSTOM");
    expect(audioEngine.play_sound).not.toHaveBeenCalled();
  });
});
