import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { createInitialGameState, type GameState } from "@pokecrystal/core/core/state";
import {
  EncounterSurface,
  WildEncounterManager,
} from "@pokecrystal/core/engine/world/overworld/wild-encounters";
import { decodeRoamerDvs, encodeRoamerDvs } from "@pokecrystal/core/engine/world/roamers";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import { createTestPokemon } from "@pokecrystal/core/engine/world/story-events/test-utils";
import type { WildEncounterData } from "@pokecrystal/assets/content/wild-encounter-data";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";

describe("Wild encounter data", () => {
  it("loads route encounters from asset data", () => {
    const loader = new DataLoader();
    loader.load_wild_encounter_data();
    expect(loader.wild_encounter_data?.has("ROUTE_29")).toBe(true);
  });

  it("resolves encounter data for route map names", () => {
    const loader = new DataLoader();
    loader.load_wild_encounter_data();
    const gameState = { wram: {} } as GameState;
    const manager = new WildEncounterManager(gameState, loader, null);
    expect(manager._lookup_map_data("Route29")).not.toBeNull();
  });

  it("loads Ice Path encounter data from ASM assets", () => {
    const loader = new DataLoader();
    loader.load_wild_encounter_data();
    const gameState = { wram: {} } as GameState;
    const manager = new WildEncounterManager(gameState, loader, null);
    const data = manager._lookup_map_data("IcePath1F");

    expect(data?.map_name).toBe("ICE_PATH_1F");
    expect(data?.grass_rates?.day).toBe(2);
    expect(data?.grass?.day).toEqual(
      expect.arrayContaining([{ level: 21, species: "SWINUB" }])
    );
  });

  it("ensures species data is available before starting a wild battle", () => {
    const loader = new DataLoader();
    loader.pokemonData.clear();
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 5)];
    const eventManager = new EventManager(gameState as unknown as GameState);
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, eventManager);

    expect(() => manager["_start_battle"]("PIDGEY", 3)).not.toThrow();

    expect(dispatchSpy).toHaveBeenCalled();
    const dispatched = dispatchSpy.mock.calls[0]?.[0];
    expect(dispatched?.name).toBe("start_battle");
    // Ensure the wild Pokemon was created from loaded species data.
    expect(dispatched?.data?.enemy_pokemon?.species?.id).toBe("PIDGEY");
  });

  it("initializes and stores roamer HP and DVs when first starting a roaming battle", () => {
    const loader = new DataLoader();
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 5)];
    gameState.wram.roaming_pokemon[0].species = "RAIKOU";
    gameState.wram.roaming_pokemon[0].level = 40;
    gameState.wram.roaming_pokemon[0].hp = 0;
    gameState.wram.roaming_pokemon[0].dvs = 0;
    const eventManager = new EventManager(gameState as unknown as GameState);
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, eventManager);

    manager["_start_battle"]("RAIKOU", 40, "BATTLETYPE_ROAMING");

    const dispatched = dispatchSpy.mock.calls.find(([event]) => event.name === "start_battle")?.[0];
    const enemy = dispatched?.data?.enemy_pokemon;
    expect(enemy?.species?.id).toBe("RAIKOU");
    expect(enemy?.hp).toBe(enemy?.max_hp);
    expect(gameState.wram.roaming_pokemon[0].hp).toBe(enemy?.max_hp & 0xff);
    expect(gameState.wram.roaming_pokemon[0].dvs).toBe(encodeRoamerDvs(enemy!.dvs));
  });

  it("reuses stored roamer HP and DVs when starting later roaming battles", () => {
    const loader = new DataLoader();
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [createTestPokemon("TOTODILE", 5)];
    const storedDvs = encodeRoamerDvs({ attack: 12, defense: 3, speed: 10, special: 5 });
    gameState.wram.roaming_pokemon[0].species = "RAIKOU";
    gameState.wram.roaming_pokemon[0].level = 40;
    gameState.wram.roaming_pokemon[0].hp = 37;
    gameState.wram.roaming_pokemon[0].dvs = storedDvs;
    const eventManager = new EventManager(gameState as unknown as GameState);
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, eventManager);

    manager["_start_battle"]("RAIKOU", 40, "BATTLETYPE_ROAMING");

    const dispatched = dispatchSpy.mock.calls.find(([event]) => event.name === "start_battle")?.[0];
    const enemy = dispatched?.data?.enemy_pokemon;
    expect(enemy?.species?.id).toBe("RAIKOU");
    expect(enemy?.hp).toBe(37);
    expect(enemy?.dvs).toEqual(decodeRoamerDvs(storedDvs));
    expect(gameState.wram.roaming_pokemon[0].hp).toBe(37);
    expect(gameState.wram.roaming_pokemon[0].dvs).toBe(storedDvs);
  });

  it("throws instead of defaulting unknown wild encounter time-of-day state to day data", () => {
    const loader = new DataLoader();
    const gameState = createInitialGameState();
    gameState.wram.time_of_day = "twilight";
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, null);

    const encounterData: WildEncounterData = {
      map_name: "TEST_MAP",
      grass_rates: { morning: 10, day: 20, night: 30 },
      water_rate: null,
      grass: {
        morning: [{ species: "RATTATA", level: 2 }],
        day: [{ species: "PIDGEY", level: 3 }],
        night: [{ species: "HOOTHOOT", level: 4 }],
      },
      water: null,
    };

    expect(() =>
      manager._resolve_table(encounterData, EncounterSurface.GRASS)
    ).toThrow("Unknown wild encounter time of day 'twilight'.");
  });

  it("throws when fallback map music resolution is missing during encounter-rate evaluation", () => {
    const loader = new DataLoader();
    const gameState = createInitialGameState();
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, null);

    expect(() =>
      manager["_current_map_music_token"]({
        current_map_name: "MISSING_WILD_MUSIC_MAP",
        audio_engine: null,
      } as never)
    ).toThrow("No default music mapping for map 'MISSING_WILD_MUSIC_MAP'.");
  });

  it.each([
    "CUT_08",
    "TALL_GRASS",
    "TALL_GRASS_10",
    "LONG_GRASS",
    "LONG_GRASS_1C",
    "CUT_28",
    "GRASS_48",
    "GRASS_49",
    "GRASS_4A",
    "GRASS_4B",
    "GRASS_4C",
  ])("treats %s collision as grass for encounter resolution", (token) => {
    const loader = new DataLoader();
    const gameState = createInitialGameState();
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, null);
    const overworld = {
      current_map_name: "Route29",
      player_x: 0,
      player_y: 0,
      map: {
        width: 1,
        height: 1,
        metatileIds: [0],
        getMetatileAt: () => 0,
      },
      tileset: {
        metatiles: [{ collision: [resolveCollisionValue(token), resolveCollisionValue(token), resolveCollisionValue(token), resolveCollisionValue(token)] }],
      },
    };

    expect(manager._resolve_surface(overworld as never)).toBe(EncounterSurface.GRASS);
  });

  it("dispatches a wild battle from Route29 grass when the encounter roll succeeds", () => {
    const loader = new DataLoader();
    loader.load_wild_encounter_data();
    const gameState = createInitialGameState();
    gameState.wram.time_of_day = "day";
    gameState.wram.wild_encounter_cooldown = 0;
    gameState.sram.party.pokemon = [createTestPokemon("CYNDAQUIL", 6), null, null, null, null, null];
    const eventManager = new EventManager(gameState as unknown as GameState);
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, eventManager, {
      rng_factory: () => ({
        nextByte: () => 0,
        randrange: () => 0,
      }),
    });
    manager.notify_step_complete();

    const overworld = {
      current_map_name: "Route29",
      player_x: 0,
      player_y: 0,
      map: {
        width: 1,
        height: 1,
        metatileIds: [0],
        getMetatileAt: () => 0,
      },
      tileset: {
        metatiles: [{ collision: [resolveCollisionValue("TALL_GRASS"), resolveCollisionValue("TALL_GRASS"), resolveCollisionValue("TALL_GRASS"), resolveCollisionValue("TALL_GRASS")] }],
      },
      audio_engine: {
        get_map_music_token: () => "MUSIC_ROUTE_29",
      },
    };

    manager.maybe_trigger_random_encounter(overworld as never);

    expect(dispatchSpy).toHaveBeenCalled();
    const dispatched = dispatchSpy.mock.calls.find(([event]) => event.name === "start_battle")?.[0];
    expect(dispatched?.name).toBe("start_battle");
    expect(dispatched?.data?.enemy_pokemon?.species?.id).toBeTruthy();
  });

  it("does not dispatch a wild battle from IcePath1F ice tiles", () => {
    const loader = new DataLoader();
    loader.load_map_attributes();
    loader.load_wild_encounter_data();
    const gameState = createInitialGameState();
    gameState.wram.time_of_day = "day";
    gameState.wram.wild_encounter_cooldown = 0;
    gameState.sram.party.pokemon = [createTestPokemon("CYNDAQUIL", 24), null, null, null, null, null];
    const eventManager = new EventManager(gameState as unknown as GameState);
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, eventManager, {
      rng_factory: () => ({
        nextByte: () => 0,
        randrange: () => 0,
      }),
    });
    manager.notify_step_complete();

    const overworld = {
      current_map_name: "IcePath1F",
      player_x: 0,
      player_y: 0,
      map: {
        width: 1,
        height: 1,
        metatileIds: [0],
        getMetatileAt: () => 0,
      },
      tileset: {
        metatiles: [{ collision: [resolveCollisionValue("ICE"), resolveCollisionValue("ICE"), resolveCollisionValue("ICE"), resolveCollisionValue("ICE")] }],
      },
      audio_engine: {
        get_map_music_token: () => "MUSIC_DARK_CAVE",
      },
    };

    manager.maybe_trigger_random_encounter(overworld as never);

    const dispatched = dispatchSpy.mock.calls.find(([event]) => event.name === "start_battle")?.[0];
    expect(dispatched).toBeUndefined();
  });

  it("dispatches a wild battle from IcePath1F cave floor when the encounter roll succeeds", () => {
    const loader = new DataLoader();
    loader.load_map_attributes();
    loader.load_wild_encounter_data();
    const gameState = createInitialGameState();
    gameState.wram.time_of_day = "day";
    gameState.wram.wild_encounter_cooldown = 0;
    gameState.sram.party.pokemon = [createTestPokemon("CYNDAQUIL", 24), null, null, null, null, null];
    const eventManager = new EventManager(gameState as unknown as GameState);
    const dispatchSpy = jest.spyOn(eventManager, "dispatch");
    const manager = new WildEncounterManager(gameState as unknown as GameState, loader, eventManager, {
      rng_factory: () => ({
        nextByte: () => 0,
        randrange: () => 0,
      }),
    });
    manager.notify_step_complete();

    const overworld = {
      current_map_name: "IcePath1F",
      player_x: 0,
      player_y: 0,
      map: {
        width: 1,
        height: 1,
        metatileIds: [0],
        getMetatileAt: () => 0,
      },
      tileset: {
        metatiles: [{ collision: [resolveCollisionValue("FLOOR"), resolveCollisionValue("FLOOR"), resolveCollisionValue("FLOOR"), resolveCollisionValue("FLOOR")] }],
      },
      audio_engine: {
        get_map_music_token: () => "MUSIC_DARK_CAVE",
      },
    };

    manager.maybe_trigger_random_encounter(overworld as never);

    const dispatched = dispatchSpy.mock.calls.find(([event]) => event.name === "start_battle")?.[0];
    expect(dispatched?.name).toBe("start_battle");
    expect(dispatched?.data?.enemy_pokemon?.species?.id).toBe("SWINUB");
  });
});
