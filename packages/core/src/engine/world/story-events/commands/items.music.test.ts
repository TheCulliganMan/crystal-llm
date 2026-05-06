import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import { PlayMapMusicCommand } from "./items";

describe("PlayMapMusicCommand", () => {
  it("routes map music through requestMapMusic when available", () => {
    const gameState = createInitialGameState();
    const requestMapMusic = jest.fn();
    const command = new PlayMapMusicCommand();

    const overworld = {
      current_map_name: "PlayersHouse1F",
      audio_engine: { requestMapMusic },
    };

    command.execute(gameState, {} as EventManager, overworld as any);

    expect(requestMapMusic).toHaveBeenCalledWith("PlayersHouse1F");
  });

  it("suppresses one restart when dont_restart_map_music is set", () => {
    const gameState = createInitialGameState();
    gameState.wram.dont_restart_map_music = true;
    const restartMapMusic = jest.fn();
    const command = new PlayMapMusicCommand();

    const overworld = {
      current_map_name: "PlayersHouse1F",
      audio_engine: { restartMapMusic },
    };

    command.execute(gameState, {} as EventManager, overworld as any);

    expect(gameState.wram.dont_restart_map_music).toBe(false);
    expect(restartMapMusic).not.toHaveBeenCalled();
  });
});
