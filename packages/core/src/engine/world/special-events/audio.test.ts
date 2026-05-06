import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { play_cur_mon_cry, play_map_music } from "./audio";

describe("play_cur_mon_cry", () => {
  it("binds pokemon cry lookups to the data loader instance", () => {
    const gameState = createInitialGameState();
    gameState.wram.wCurPartySpecies = "TOTODILE";
    const audioEngine = {
      playSound: jest.fn(),
    } as unknown as AudioEngine;
    const dataLoader = {
      cryMap: new Map([["TOTODILE", { cry_id: "CRY_CUSTOM" }]]),
      getPokemonCry(name: string) {
        if (this !== dataLoader) {
          throw new Error("unbound cry lookup");
        }
        return this.cryMap.get(name) ?? null;
      },
    };

    const result = play_cur_mon_cry(gameState, {
      overworld: { data_loader: dataLoader, audio_engine: audioEngine },
    });

    expect(result).toBe("CRY_CUSTOM");
    expect(audioEngine.playSound).toHaveBeenCalledWith("CRY_CUSTOM");
  });
});

describe("play_map_music", () => {
  it("routes map playback through requestMapMusic when available", () => {
    const gameState = createInitialGameState();
    const audioEngine = { playMusic: jest.fn() } as unknown as AudioEngine;
    const requestMapMusic = jest.fn();

    const result = play_map_music(gameState, {
      overworld: { current_map_name: "PlayersHouse1F", requestMapMusic },
      audio_engine: audioEngine,
    });

    expect(result).toBe(true);
    expect(requestMapMusic).toHaveBeenCalledWith("PlayersHouse1F");
  });

  it("suppresses one restart when dont_restart_map_music is set", () => {
    const gameState = createInitialGameState();
    gameState.wram.dont_restart_map_music = true;
    const restartMapMusic = jest.fn();
    const audioEngine = { playMusic: jest.fn() } as unknown as AudioEngine;

    const result = play_map_music(gameState, {
      overworld: { current_map_name: "PlayersHouse1F", restartMapMusic },
      audio_engine: audioEngine,
    });

    expect(result).toBe(true);
    expect(restartMapMusic).toHaveBeenCalledTimes(1);
  });
});
