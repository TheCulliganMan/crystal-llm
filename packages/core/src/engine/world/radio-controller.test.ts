import { createInitialGameState } from "@pokecrystal/core/core/state";
import { Event, EventManager } from "@pokecrystal/core/engine/events/events";
import { RadioEventController } from "./radio";

describe("RadioEventController", () => {
  it("routes radio start through the audio controller", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const startRadioChannel = jest.fn();
    const controller = new RadioEventController({
      eventManager,
      audioEngine: { startRadioChannel, stopRadioChannel: jest.fn() } as any,
    });
    controller.register();

    eventManager.dispatch(
      new Event("play_radio_channel", {
        station: "POKE_FLUTE_RADIO",
        duration_frames: 120,
      }),
    );

    expect(startRadioChannel).toHaveBeenCalledWith("POKE_FLUTE_RADIO", 120);
  });

  it("throws for an unknown radio station instead of warning and continuing", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const controller = new RadioEventController({
      eventManager,
      audioEngine: { startRadioChannel: jest.fn(), stopRadioChannel: jest.fn() } as any,
    });
    controller.register();

    expect(() =>
      (controller as any).handlePlayRadioChannel(
        new Event("play_radio_channel", {
          station: "NOT_A_REAL_STATION",
        }),
        gameState,
      ),
    ).toThrow("Missing radio station 'NOT_A_REAL_STATION'.");
  });

  it("throws when a configured radio station is missing a song mapping", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const controller = new RadioEventController({
      eventManager,
      audioEngine: { startRadioChannel: jest.fn(), stopRadioChannel: jest.fn() } as any,
    });
    controller.register();
    (controller as any).stationInfo.set("POKE_FLUTE_RADIO", {
      constant: "POKE_FLUTE_RADIO",
      id: 8,
      song: "",
    });

    expect(() =>
      (controller as any).handlePlayRadioChannel(
        new Event("play_radio_channel", {
          station: "POKE_FLUTE_RADIO",
        }),
        gameState,
      ),
    ).toThrow("Radio station 'POKE_FLUTE_RADIO' is missing a song mapping.");
  });
});
