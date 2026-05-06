import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { OverworldEngine } from "./overworld";

describe("OverworldEngine.start_map_music", () => {
  it("queues map-music requests through the audio controller", () => {
    const gameState = createInitialGameState();
    const requestMapMusic = jest.fn();

    const overworld = {
      audio_controller: { requestMapMusic },
      current_map_name: "PlayersHouse1F",
      game_state: gameState,
    } as unknown as OverworldEngine;

    OverworldEngine.prototype.start_map_music.call(overworld);

    expect(requestMapMusic).toHaveBeenCalledWith("PlayersHouse1F", undefined);
  });

  it("prioritizes bike music when the player is biking", () => {
    const gameState = createInitialGameState();
    const requestMapMusic = jest.fn();

    const overworld = {
      audio_controller: { requestMapMusic },
      current_map_name: "PlayersHouse1F",
      game_state: gameState,
      player_state: PlayerState.BIKE,
    } as unknown as OverworldEngine;

    OverworldEngine.prototype.start_map_music.call(overworld);

    expect(requestMapMusic).toHaveBeenCalledWith("PlayersHouse1F", PlayerState.BIKE);
  });
});

describe("OverworldEngine.restart_map_music", () => {
  it("routes restarts through the audio controller", () => {
    const gameState = createInitialGameState();
    const restartMapMusic = jest.fn();

    const overworld = {
      audio_controller: { restartMapMusic },
      game_state: gameState,
    } as unknown as OverworldEngine;

    OverworldEngine.prototype.restart_map_music.call(overworld);

    expect(restartMapMusic).toHaveBeenCalledTimes(1);
  });

  it("suppresses one restart when dont_restart_map_music is set", () => {
    const gameState = createInitialGameState();
    gameState.wram.dont_restart_map_music = true;
    const restartMapMusic = jest.fn();

    const overworld = {
      audio_controller: null,
      audio_engine: { restartMapMusic },
      game_state: gameState,
    } as unknown as OverworldEngine;

    OverworldEngine.prototype.restart_map_music.call(overworld);

    expect(gameState.wram.dont_restart_map_music).toBe(false);
    expect(restartMapMusic).not.toHaveBeenCalled();
  });
});
