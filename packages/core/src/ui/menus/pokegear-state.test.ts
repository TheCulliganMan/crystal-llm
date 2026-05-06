import { createInitialGameState } from "@pokecrystal/core/core/state";
import { getMapMetadataByConstant } from "@pokecrystal/core/engine/world/maps";

const setMap = (gameState: ReturnType<typeof createInitialGameState>, mapConstant: string): void => {
  const metadata = getMapMetadataByConstant(mapConstant);
  if (!metadata) {
    throw new Error(`Missing map metadata for ${mapConstant}`);
  }
  gameState.wram.wMapGroup = metadata.groupId;
  gameState.wram.wMapNumber = metadata.mapId;
};

describe("PokegearStateMachine radio station names", () => {
  afterEach(() => {
    jest.resetModules();
    jest.dontMock("@pokecrystal/assets/content/radio");
  });

  it("throws when a station constant mapping is missing instead of silently dropping the station", () => {
    jest.doMock("@pokecrystal/assets/content/radio", () => {
      const actual = jest.requireActual("@pokecrystal/assets/content/radio");
      return {
        ...actual,
        RADIO_CHANNEL_CONSTANTS: actual.RADIO_CHANNEL_CONSTANTS.filter(
          (entry: { constant: string }) => entry.constant !== "OAKS_POKEMON_TALK",
        ),
      };
    });

    jest.isolateModules(() => {
      const gameState = createInitialGameState();
      gameState.wram.time_of_day = "nite";
      const { PokegearStateMachine } = require("./pokegear-state");
      const logic = new PokegearStateMachine(gameState);

      expect(() => logic.currentRadioStation()).toThrow(
        "Missing radio station channel info for 'OAKS_POKEMON_TALK'.",
      );
    });
  });

  it("throws when a station name mapping is missing instead of synthesizing a title-cased fallback", () => {
    jest.doMock("@pokecrystal/assets/content/radio", () => {
      const actual = jest.requireActual("@pokecrystal/assets/content/radio");
      const stationNames = { ...actual.RADIO_STATION_NAMES } as Record<string, string>;
      delete stationNames.OaksPKMNTalkName;
      return {
        ...actual,
        RADIO_STATION_NAMES: stationNames,
      };
    });

    jest.isolateModules(() => {
      const gameState = createInitialGameState();
      gameState.wram.time_of_day = "nite";
      const { PokegearStateMachine } = require("./pokegear-state");
      const logic = new PokegearStateMachine(gameState);

      expect(() => logic.currentRadioStation()).toThrow(
        "Missing radio station name mapping for label 'OaksPKMNTalkName'.",
      );
    });
  });

  it("throws when SRAM phone numbers contain an unknown phone contact id", () => {
    const gameState = createInitialGameState();
    gameState.sram.phone_numbers = ["PHONE_FAKE_CONTACT"];
    const { PokegearStateMachine } = require("./pokegear-state");

    expect(() => new PokegearStateMachine(gameState)).toThrow(
      "Unknown phone contact 'PHONE_FAKE_CONTACT' in SRAM phone list.",
    );
  });

  it("accepts the shipped Mom phone contact in SRAM without throwing", () => {
    const gameState = createInitialGameState();
    gameState.sram.phone_numbers = ["PHONE_MOM"];
    const { PokegearStateMachine } = require("./pokegear-state");

    expect(() => new PokegearStateMachine(gameState)).not.toThrow();
  });

  it("uses the weekday Pokemon March/Lullaby music for Pokemon Music", () => {
    const gameState = createInitialGameState();
    const { PokegearStateMachine } = require("./pokegear-state");
    const logic = new PokegearStateMachine(gameState);

    logic.setRadioIndex(1);
    gameState.sram.day_of_week = 0;
    expect(logic.currentRadioStation()?.song).toBe("MUSIC_POKEMON_MARCH");

    gameState.sram.day_of_week = 1;
    expect(logic.currentRadioStation()?.song).toBe("MUSIC_POKEMON_LULLABY");
  });

  it("overrides Johto Pokegear broadcasts with Team Rocket during the radio tower takeover", () => {
    const gameState = createInitialGameState();
    gameState.wram.engine_flags.ENGINE_ROCKETS_IN_RADIO_TOWER = true;
    setMap(gameState, "NEW_BARK_TOWN");
    const { PokegearStateMachine } = require("./pokegear-state");
    const logic = new PokegearStateMachine(gameState);

    expect(logic.currentRadioStation()?.constant).toBe("ROCKET_RADIO");
    expect(logic.currentRadioStation()?.song).toBe("MUSIC_ROCKET_OVERTURE");
  });

  it("does not apply the Team Rocket override to Kanto broadcasts", () => {
    const gameState = createInitialGameState();
    gameState.wram.engine_flags.ENGINE_ROCKETS_IN_RADIO_TOWER = true;
    gameState.wram.engine_flags.ENGINE_EXPN_CARD = true;
    setMap(gameState, "PALLET_TOWN");
    const { PokegearStateMachine } = require("./pokegear-state");
    const logic = new PokegearStateMachine(gameState);

    logic.setRadioIndex(5);
    expect(logic.currentRadioStation()?.constant).toBe("PLACES_AND_PEOPLE");
  });

  it("requires the real engine expansion card flag for Kanto broadcasts", () => {
    const gameState = createInitialGameState();
    gameState.wram.event_flags.ENGINE_EXPN_CARD = true;
    gameState.wram.engine_flags.ENGINE_EXPN_CARD = false;
    setMap(gameState, "PALLET_TOWN");
    const { PokegearStateMachine } = require("./pokegear-state");
    const logic = new PokegearStateMachine(gameState);

    logic.setRadioIndex(5);
    expect(logic.currentRadioStation()).toBeNull();

    gameState.wram.engine_flags.ENGINE_EXPN_CARD = true;
    expect(logic.currentRadioStation()?.constant).toBe("PLACES_AND_PEOPLE");
  });
});
