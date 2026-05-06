import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/events/events";
import {
  createScriptRunnerStub,
  createTestPokemon,
} from "@pokecrystal/core/engine/world/story-events/test-utils";
import { TextFormatter } from "@pokecrystal/core/engine/world/story-events/text-formatter";
import { move_tutor, name_rater, poke_seer } from "./services";

describe("special event services", () => {
  it("throws instead of rewriting a malformed selected party index to the first party slot", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", 152, { nickname: "LEAF" }),
    ];
    const runner = createScriptRunnerStub({
      game_state: gameState,
      variables: { _selected_party_index: "NOT_A_PARTY_INDEX" },
    });

    expect(() => name_rater(gameState, { runner })).toThrow(
      "Invalid runner index '_selected_party_index': NOT_A_PARTY_INDEX",
    );
  });

  it("does not rewrite a malformed move slot when MoveTutor exits early", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", 152, {
        moves: [
          { name: "TACKLE", current_pp: 35 },
          { name: "LEER", current_pp: 30 },
        ],
      }),
    ];
    const runner = createScriptRunnerStub({
      game_state: gameState,
      variables: {
        _selected_party_index: 0,
        _selected_move: "CUT",
        _selected_move_index: "BAD_MOVE_SLOT",
      },
    });

    expect(() => move_tutor(gameState, { runner })).not.toThrow();
  });

  it("queues the PokeSeer ASM intro before waiting for party selection", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({
      game_state: gameState,
      event_manager: eventManager,
      variables: { _selected_party_index: 0 },
    });
    gameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", 152, { nickname: "LEAF" }),
    ];
    const events: Array<{ name: string; text?: string }> = [];
    eventManager.on("show_text", (event) => {
      events.push({ name: event.name, text: String(event.data.text ?? "") });
    });
    eventManager.on("wait_for_input", (event) => {
      events.push({ name: event.name });
    });

    poke_seer(gameState, { runner, event_manager: eventManager });

    expect(events[0]).toMatchObject({
      name: "show_text",
      text: expect.stringContaining("I see all."),
    });
    expect(events[1]).toMatchObject({ name: "wait_for_input" });
    expect(events.some((event) => event.text?.includes("Whaaaat? I can't"))).toBe(true);
  });

  it("prints the PokeSeer cancel text when party selection is cancelled", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const runner = createScriptRunnerStub({
      game_state: gameState,
      event_manager: eventManager,
      variables: { _selection_cancelled: true },
    });
    gameState.sram.party.pokemon = [
      createTestPokemon("CHIKORITA", 152, { nickname: "LEAF" }),
    ];
    const texts: string[] = [];
    eventManager.on("show_text", (event) => {
      texts.push(String(event.data.text ?? ""));
    });

    poke_seer(gameState, { runner, event_manager: eventManager });

    expect(texts).toEqual([
      expect.stringContaining("I see all."),
      expect.stringContaining("Fufufu! I saw that"),
    ]);
    expect(runner.last_condition_result).toBe(false);
  });

  it("formats PokeSeer met-data text through ASM text buffers", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_id = 77;
    const eventManager = new EventManager(gameState);
    const formatter = new TextFormatter(gameState);
    const runner = createScriptRunnerStub({
      game_state: gameState,
      event_manager: eventManager,
      variables: { _selected_party_index: 0 },
      formatText: (text) => formatter.formatText(text),
    });
    Object.defineProperty(runner, "string_buffers", {
      get: () => formatter.stringBuffers,
      set: (value: Record<string, string>) => {
        formatter.stringBuffers = value;
      },
    });
    const mon = createTestPokemon("CHIKORITA", 152, {
      nickname: "LEAF",
      level: 16,
      original_trainer_id: 77,
    });
    Object.assign(mon as unknown as Record<string, unknown>, {
      met_level: 5,
      met_location: "NEW BARK TOWN",
      met_time: "Morning",
    });
    gameState.sram.party.pokemon = [mon];
    const texts: string[] = [];
    eventManager.on("show_text", (event) => {
      texts.push(String(event.data.text ?? ""));
    });

    poke_seer(gameState, { runner, event_manager: eventManager });

    const joined = texts.join("\n");
    expect(joined).toContain("LEAF here:");
    expect(joined).toContain("NEW BARK TOWN");
    expect(joined).toContain("The time was\nMorning!");
    expect(joined).toContain("Its level was 5!");
    expect(joined).toContain("LEAF seems");
    expect(joined).not.toContain("@");
  });
});
