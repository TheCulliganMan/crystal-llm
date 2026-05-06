import { createInitialGameState } from "@pokecrystal/core/core/state";
import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";
import { OverworldAudioController } from "./audio-controller";

const createTransport = () => {
  const playing = new Set<string>();
  return {
    playing,
    transport: {
      update: jest.fn(),
      playMusic: jest.fn(),
      play_music: jest.fn(),
      stopMusic: jest.fn(),
      stop_music: jest.fn(),
      playSound: jest.fn((token: string) => {
        playing.add(token);
      }),
      play_sound: jest.fn((token: string) => {
        playing.add(token);
      }),
      isSoundPlaying: jest.fn((token?: string) => {
        if (!token) {
          return playing.size > 0;
        }
        return playing.has(token);
      }),
      fadeOutMusicFrames: jest.fn(),
      fadeToMusicFrames: jest.fn(),
      setMusicMutedByController: jest.fn(),
      canResolveMusicToken: jest.fn(() => true),
      canResolveSoundToken: jest.fn(() => true),
    },
  };
};

const setMap = (gameState: ReturnType<typeof createInitialGameState>, mapConstant: string): void => {
  const metadata = getMapMetadataByConstant(mapConstant);
  if (!metadata) {
    throw new Error(`Missing map metadata for ${mapConstant}`);
  }
  gameState.wram.wMapGroup = metadata.groupId;
  gameState.wram.wMapNumber = metadata.mapId;
};

describe("OverworldAudioController", () => {
  it("starts map music once and does not restart when already active", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();
    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();

    expect(transport.playMusic).toHaveBeenCalledTimes(1);
    expect(transport.playMusic).toHaveBeenCalledWith("MUSIC_NEW_BARK_TOWN", "map");
    expect(gameState.wram.wMapMusic).toBe("MUSIC_NEW_BARK_TOWN");
  });

  it("prioritizes bike music and restores the map song after biking ends", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.BIKE);
    controller.update();
    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.restartMapMusic();
    controller.update();

    expect(transport.playMusic).toHaveBeenNthCalledWith(1, "MUSIC_BICYCLE", "map");
    expect(transport.playMusic).toHaveBeenNthCalledWith(2, "MUSIC_NEW_BARK_TOWN", "map");
  });

  it("suppresses one map restart when dont_restart_map_music is set", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();

    gameState.wram.dont_restart_map_music = true;
    controller.requestEncounterMusic("LASS");
    controller.update();
    controller.restartMapMusic();
    controller.update();
    controller.restartMapMusic();
    controller.update();

    expect(transport.playMusic).toHaveBeenNthCalledWith(1, "MUSIC_NEW_BARK_TOWN", "map");
    expect(transport.playMusic).toHaveBeenNthCalledWith(2, "MUSIC_LASS_ENCOUNTER", "encounter");
    expect(transport.playMusic).toHaveBeenNthCalledWith(3, "MUSIC_NEW_BARK_TOWN", "map");
    expect(gameState.wram.dont_restart_map_music).toBe(false);
  });

  it("restores map music after a timed radio takeover expires", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();
    controller.startRadioChannel("POKE_FLUTE_RADIO", 2);
    controller.update();
    controller.update();
    controller.update();

    expect(transport.playMusic).toHaveBeenNthCalledWith(1, "MUSIC_NEW_BARK_TOWN", "map");
    expect(transport.playMusic).toHaveBeenNthCalledWith(2, "MUSIC_POKE_FLUTE_CHANNEL", "radio");
    expect(transport.playMusic).toHaveBeenNthCalledWith(3, "MUSIC_NEW_BARK_TOWN", "map");
  });

  it("writes active radio music to wMapMusic for script specials", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("VermilionCity", PlayerState.NORMAL);
    controller.update();
    controller.startRadioChannel("POKE_FLUTE_RADIO", 0);
    controller.update();

    expect(transport.playMusic).toHaveBeenNthCalledWith(1, "MUSIC_VERMILION_CITY", "map");
    expect(transport.playMusic).toHaveBeenNthCalledWith(2, "MUSIC_POKE_FLUTE_CHANNEL", "radio");
    expect(gameState.wram.wMapMusic).toBe("MUSIC_POKE_FLUTE_CHANNEL");
  });

  it("updates wMapMusic as soon as a radio station starts", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("VermilionCity", PlayerState.NORMAL);
    controller.update();
    controller.startRadioChannel("POKE_FLUTE_RADIO", 0);

    expect(gameState.wram.wMapMusic).toBe("MUSIC_POKE_FLUTE_CHANNEL");
  });

  it("restores active radio from saved wMapMusic before map music can overwrite it", () => {
    const gameState = createInitialGameState();
    gameState.wram.wMapMusic = "MUSIC_POKE_FLUTE_CHANNEL";
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("VermilionCity", PlayerState.NORMAL);
    controller.update();

    expect(transport.playMusic).toHaveBeenCalledWith("MUSIC_POKE_FLUTE_CHANNEL", "radio");
    expect(gameState.wram.wMapMusic).toBe("MUSIC_POKE_FLUTE_CHANNEL");
  });

  it("routes cries through the audio transport without controller-side mute bookkeeping", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();
    controller.playCry("CRY_TOTODILE");
    controller.update();

    expect(transport.playSound).toHaveBeenCalledWith("CRY_TOTODILE");
    expect(transport.setMusicMutedByController).not.toHaveBeenCalled();
  });

  it("delegates fade scheduling to the transport when engine-managed fades exist", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();
    controller.fadeToMusic("MUSIC_ROUTE_30", 2, "general");
    expect(transport.fadeToMusicFrames).toHaveBeenCalledWith("MUSIC_ROUTE_30", 2, "general");
    expect(transport.fadeOutMusicFrames).not.toHaveBeenCalled();
  });

  it("stops music cleanly for silence tokens without clearing map state", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();
    controller.requestMusic("MUSIC_NONE", "general");
    controller.update();

    expect(transport.stopMusic).toHaveBeenCalledTimes(1);
    expect(gameState.wram.wMapMusic).toBe("MUSIC_NEW_BARK_TOWN");
  });

  it("reports temporary script music overrides until map music is restored", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();
    expect(controller.hasTemporaryMusicOverride()).toBe(false);

    controller.requestMusic("MUSIC_MOM", "general");
    controller.update();
    expect(controller.hasTemporaryMusicOverride()).toBe(true);

    controller.restartMapMusic();
    controller.update();
    expect(controller.hasTemporaryMusicOverride()).toBe(false);
    expect(transport.playMusic).toHaveBeenNthCalledWith(2, "MUSIC_MOM", "general");
    expect(transport.playMusic).toHaveBeenNthCalledWith(3, "MUSIC_NEW_BARK_TOWN", "map");
  });

  it("replays map music after battle music bypasses the controller", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    controller.requestMapMusic("PlayersHouse1F", PlayerState.NORMAL);
    controller.update();
    transport.playMusic.mockClear();

    transport.playMusic("MUSIC_JOHTO_WILD_BATTLE", "battle");
    transport.playMusic.mockClear();

    controller.restartMapMusic();
    controller.update();

    expect(transport.playMusic).toHaveBeenCalledWith("MUSIC_NEW_BARK_TOWN", "map");
  });

  it("throws for missing radio mappings", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    expect(() => controller.startRadioChannel("NOT_A_REAL_STATION", 5)).toThrow(
      "Radio station 'NOT_A_REAL_STATION' is missing a song mapping.",
    );
  });

  it("plays Pokemon Music as weekday-specific March or Lullaby radio", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);

    gameState.sram.day_of_week = 0;
    controller.startRadioChannel("POKEMON_MUSIC", 0);
    controller.update();

    gameState.sram.day_of_week = 1;
    controller.startRadioChannel("POKEMON_MUSIC", 0);
    controller.update();

    expect(transport.playMusic).toHaveBeenNthCalledWith(1, "MUSIC_POKEMON_MARCH", "radio");
    expect(transport.playMusic).toHaveBeenNthCalledWith(2, "MUSIC_POKEMON_LULLABY", "radio");
  });

  it("routes Johto radio through Team Rocket during the radio tower takeover", () => {
    const gameState = createInitialGameState();
    const { transport } = createTransport();
    const controller = new OverworldAudioController(gameState, transport);
    gameState.wram.engine_flags.ENGINE_ROCKETS_IN_RADIO_TOWER = true;
    setMap(gameState, "NEW_BARK_TOWN");

    controller.startRadioChannel("LUCKY_CHANNEL", 0);
    controller.update();

    expect(transport.playMusic).toHaveBeenCalledWith("MUSIC_ROCKET_OVERTURE", "radio");
  });
});
