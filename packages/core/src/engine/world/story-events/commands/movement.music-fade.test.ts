import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import type { EventManager } from "@pokecrystal/core/engine/events/events";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { MusicFadeOutCommand, PlayMusicCommand } from "./movement";

describe("PlayMusicCommand", () => {
  it("routes overworld script music through the controller host", () => {
    const gameState = createInitialGameState();
    const requestMusic = jest.fn();
    const playMusic = jest.fn();
    const command = new PlayMusicCommand("MUSIC_SHOW_ME_AROUND");

    const overworld = {
      requestMusic,
      audio_engine: {
        playMusic,
      },
    };

    command.execute(gameState, {} as EventManager, overworld as any);

    expect(requestMusic).toHaveBeenCalledWith("MUSIC_SHOW_ME_AROUND", "general");
    expect(playMusic).not.toHaveBeenCalled();
  });

  it("uses the general role for direct playback fallback and does not overwrite map music", () => {
    const gameState = createInitialGameState();
    gameState.wram.wMapMusic = "MUSIC_NEW_BARK_TOWN";
    const playMusic = jest.fn();
    const command = new PlayMusicCommand("MUSIC_CREDITS");

    const overworld = {
      game_state: gameState,
      audio_engine: {
        playMusic,
      },
    };

    command.execute(gameState, {} as EventManager, overworld as any);

    expect(playMusic).toHaveBeenCalledWith("MUSIC_CREDITS", "general");
    expect(gameState.wram.wMapMusic).toBe("MUSIC_NEW_BARK_TOWN");
  });
});

describe("MusicFadeOutCommand", () => {
  it("routes overworld fade music through the controller host", () => {
    const gameState = createInitialGameState();
    const fadeToMusic = jest.fn();
    const command = new MusicFadeOutCommand("MUSIC_ROUTE_30", 120);

    const overworld = {
      fadeToMusic,
      audio_engine: {
        fadeToMusic: jest.fn(),
      },
    };

    command.execute(gameState, {} as EventManager, overworld as any);

    expect(fadeToMusic).toHaveBeenCalledWith("MUSIC_ROUTE_30", 120, "general");
    expect(overworld.audio_engine.fadeToMusic).not.toHaveBeenCalled();
  });

  it("uses Game Boy frame timing when falling back to fadeOutMusic milliseconds", () => {
    const gameState = createInitialGameState();
    const fadeOutMusic = jest.fn();
    const playMusic = jest.fn();
    const command = new MusicFadeOutCommand("MUSIC_ROUTE_30", 120);

    const overworld = {
      audio_engine: {
        fadeOutMusic,
        playMusic,
      },
    };

    command.execute(gameState, {} as EventManager, overworld as any);

    expect(fadeOutMusic).toHaveBeenCalledTimes(1);
    expect(fadeOutMusic).toHaveBeenCalledWith(Math.round(120 * GB_FRAME_DURATION_MS));
    expect(playMusic).toHaveBeenCalledWith("MUSIC_ROUTE_30", "general");
  });
});
