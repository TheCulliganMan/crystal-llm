import {
  HALL_OF_FAME_TEAM_SIZE,
  HOF_MASTER_COUNT,
  NUM_HALL_OF_FAME_ENTRIES,
  Pokemon as PokemonId,
} from "@pokecrystal/core/core/constants";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager, type StartCreditsEvent } from "@pokecrystal/core/engine/events/events";
import {
  createOverworldStub,
  createScriptRunnerStub,
  createTestPokemon,
} from "@pokecrystal/core/engine/world/story-events/test-utils";
import { HallOfFameCommand } from "./hall-of-fame";

describe("HallOfFameCommand", () => {
  it("records the current non-egg party with packed data and fixed-size padding", () => {
    const gameState = createInitialGameState();
    const lead = createTestPokemon("CYNDAQUIL", PokemonId.CYNDAQUIL, {
      nickname: "EGG",
      level: 44,
      original_trainer_id: 0x1234,
      dvs: { attack: 1, defense: 2, speed: 3, special: 4, hp: 0 },
    });
    const egg = createTestPokemon("EGG", PokemonId.EGG, { nickname: "EGG" });
    gameState.sram.party.pokemon = [lead, egg, null, null, null, null];
    gameState.wram.wPartyCount = 2;
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({ game_state: gameState, event_manager: eventManager });
    const command = new HallOfFameCommand();

    command.runner = runner;
    command.execute(gameState, eventManager, createOverworldStub());

    expect(gameState.sram.hall_of_fame).toHaveLength(1);
    expect(gameState.sram.hall_of_fame[0].win_count).toBe(1);
    expect(gameState.sram.hall_of_fame[0].team).toHaveLength(HALL_OF_FAME_TEAM_SIZE);
    expect(gameState.sram.hall_of_fame[0].team[0]).toEqual({
      species: "CYNDAQUIL",
      id: PokemonId.CYNDAQUIL,
      trainer_id: 0x1234,
      dvs: 0x1234,
      level: 44,
      nickname: "EGG",
    });
    expect(gameState.sram.hall_of_fame[0].team[1]).toEqual({});
    expect(gameState.wram.engine_flags.STATUSFLAGS_HALL_OF_FAME_F).toBe(true);
    expect(runner.last_value).toEqual({
      hall_of_fame: {
        win_count: 1,
        team: ["EGG", "", "", "", "", ""],
        total_entries: 1,
      },
    });
  });

  it("saturates the Hall of Fame count and keeps the newest thirty entries", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", PokemonId.TOTODILE),
      null,
      null,
      null,
      null,
      null,
    ];
    gameState.wram.wPartyCount = 1;
    gameState.wram.wHallOfFameCount = HOF_MASTER_COUNT - 1;
    gameState.sram.hall_of_fame = Array.from({ length: NUM_HALL_OF_FAME_ENTRIES }, (_, index) => ({
      win_count: index + 1,
      team: [{ species: `OLD${index}` }],
      pokemon: [{ species: `OLD${index}` }],
    }));
    const eventManager = new EventManager(gameState);

    new HallOfFameCommand().execute(gameState, eventManager, createOverworldStub());

    expect(gameState.wram.wHallOfFameCount).toBe(HOF_MASTER_COUNT);
    expect(gameState.sram.hall_of_fame).toHaveLength(NUM_HALL_OF_FAME_ENTRIES);
    expect(gameState.sram.hall_of_fame[0].win_count).toBe(HOF_MASTER_COUNT);
    expect(gameState.sram.hall_of_fame[0].team[0].species).toBe("TOTODILE");
    expect(gameState.sram.hall_of_fame.at(-1)?.team[0].species).toBe("OLD28");

    new HallOfFameCommand().execute(gameState, eventManager, createOverworldStub());

    expect(gameState.wram.wHallOfFameCount).toBe(HOF_MASTER_COUNT);
    expect(gameState.sram.hall_of_fame[0].win_count).toBe(HOF_MASTER_COUNT);
  });

  it("only allows credit skipping after a prior Hall of Fame record", () => {
    const firstGameState = createInitialGameState();
    firstGameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", PokemonId.CHIKORITA),
      null,
      null,
      null,
      null,
      null,
    ];
    firstGameState.wram.wPartyCount = 1;
    const firstEventManager = new EventManager(firstGameState);
    const firstCredits: boolean[] = [];
    firstEventManager.on("start_credits", (event: StartCreditsEvent) => {
      firstCredits.push(event.data.allow_skip);
    });

    new HallOfFameCommand().execute(firstGameState, firstEventManager, createOverworldStub());

    expect(firstCredits).toEqual([false]);

    const repeatGameState = createInitialGameState();
    repeatGameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", PokemonId.CHIKORITA),
      null,
      null,
      null,
      null,
      null,
    ];
    repeatGameState.wram.wPartyCount = 1;
    repeatGameState.wram.wHallOfFameCount = 1;
    const repeatEventManager = new EventManager(repeatGameState);
    const repeatCredits: boolean[] = [];
    repeatEventManager.on("start_credits", (event: StartCreditsEvent) => {
      repeatCredits.push(event.data.allow_skip);
    });

    new HallOfFameCommand().execute(repeatGameState, repeatEventManager, createOverworldStub());

    expect(repeatCredits).toEqual([true]);

    const legacyGameState = createInitialGameState();
    legacyGameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", PokemonId.CHIKORITA),
      null,
      null,
      null,
      null,
      null,
    ];
    legacyGameState.sram.hall_of_fame = [{ win_count: 1, team: [{ species: "OLD" }] }];
    legacyGameState.wram.wPartyCount = 1;
    legacyGameState.wram.wHallOfFameCount = 0;
    const legacyEventManager = new EventManager(legacyGameState);
    const legacyCredits: boolean[] = [];
    legacyEventManager.on("start_credits", (event: StartCreditsEvent) => {
      legacyCredits.push(event.data.allow_skip);
    });

    new HallOfFameCommand().execute(legacyGameState, legacyEventManager, createOverworldStub());

    expect(legacyCredits).toEqual([true]);
  });
});
