import { createInitialGameState } from "@pokecrystal/core/core/state";
import {
  createOverworldEngineStub,
  createScriptRunnerStub,
  createTestPokemon,
} from "@pokecrystal/core/engine/world/story-events/test-utils";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { Surface } from "@pokecrystal/core/ui/surface";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import * as asyncLoop from "@pokecrystal/core/ui/async-loop";
import {
  daisys_grooming,
  move_deletion,
  name_rival,
  older_haircut_brother,
  photo_studio,
  prof_oaks_pc_boot,
  younger_haircut_brother,
} from "./placeholders";

class NameEntryUiStub extends BaseUI {
  public readonly eventQueue = gameEngine.event.createQueue();
  public readonly updateMock = jest.fn();
  public readonly font = { renderText: jest.fn() };

  constructor() {
    super(160, 144, 1);
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  public update(): void {
    this.updateMock();
  }
}

describe("name_rival", () => {
  it("uses the runner override when provided", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ variables: { _rival_name: "BLUE" } });

    const result = name_rival(gameState, { runner });

    expect(result).toBe("BLUE");
    expect(gameState.sram.rival_name).toBe("BLUE");
    expect(runner.last_value).toBe("BLUE");
    expect(runner.last_condition_result).toBe(true);
  });

  it("runs the naming screen when UI is available", async () => {
    const events = [
      [new gameEngine.event.Event("keydown", { text: "R" })],
      [new gameEngine.event.Event("keydown", { text: "E" })],
      [new gameEngine.event.Event("keydown", { text: "D" })],
      [new gameEngine.event.Event("keydown", { code: "Enter" })],
      [new gameEngine.event.Event("keydown", { code: "KeyZ" })],
    ];
    const getEvents = jest.spyOn(gameEngine.event, "get").mockImplementation(() => events.shift() ?? []);
    const nextFrameMock = jest.spyOn(asyncLoop, "nextFrame").mockResolvedValue(undefined);

    const ui = new NameEntryUiStub();
    const overworld = createOverworldEngineStub({ ui, input_capture_active: false });
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ overworld });

    try {
      const result = await name_rival(gameState, { runner, overworld });

      expect(result).toBe("RED");
      expect(gameState.sram.rival_name).toBe("RED");
      expect(overworld.input_capture_active).toBe(false);
      expect(runner.last_value).toBe("RED");
      expect(runner.last_condition_result).toBe(true);
      expect(getEvents).toHaveBeenCalled();
    } finally {
      getEvents.mockRestore();
      nextFrameMock.mockRestore();
    }
  });
});

describe("haircut specials", () => {
  it("returns the ASM older-brother outcome code and updates the selected party mon happiness", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("EEVEE", 133, { nickname: "BUDDY", happiness: 50 }),
    ];
    const runner = createScriptRunnerStub({
      variables: { _selected_party_index: 0, _rng_roll: 76 },
    });

    const result = older_haircut_brother(gameState, { runner });

    expect(result).toBe(3);
    expect(runner.last_value).toBe(3);
    expect(gameState.sram.party.pokemon[0]?.happiness).toBe(53);
    expect(gameState.wram.wCurPartySpecies).toBe("EEVEE");
    expect(runner.string_buffers.STRING_BUFFER_1).toBe("BUDDY");
    expect(runner.string_buffers.STRING_BUFFER_3).toBe("BUDDY");
  });

  it("returns 1 for eggs without applying a haircut outcome", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("EGG", 0, { happiness: 80 }),
    ];
    const runner = createScriptRunnerStub({
      variables: { _selected_party_index: 0, _rng_roll: 0 },
    });

    const result = younger_haircut_brother(gameState, { runner });

    expect(result).toBe(1);
    expect(runner.last_value).toBe(1);
    expect(gameState.sram.party.pokemon[0]?.happiness).toBe(80);
    expect(runner.string_buffers.STRING_BUFFER_1).toBeUndefined();
    expect(runner.string_buffers.STRING_BUFFER_3).toBeUndefined();
  });

  it("returns 0 when party selection is cancelled", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("PIKACHU", 25, { happiness: 70 }),
    ];
    const runner = createScriptRunnerStub({
      variables: { _selection_cancelled: true },
    });

    const result = daisys_grooming(gameState, { runner });

    expect(result).toBe(0);
    expect(runner.last_value).toBe(0);
    expect(runner.last_condition_result).toBe(false);
    expect(gameState.sram.party.pokemon[0]?.happiness).toBe(70);
  });
});

describe("move_deletion", () => {
  it("returns falsey when selection is cancelled", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({ variables: { _selection_cancelled: true } });

    const result = move_deletion(gameState, { runner });

    expect(result).toBe("");
    expect(runner.last_value).toBe("");
    expect(runner.last_condition_result).toBe(false);
  });

  it("requires an explicit move selection instead of fabricating TACKLE", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("CYNDAQUIL", 155, {
        moves: [
          { name: "SCRATCH", current_pp: 35 },
          { name: "LEER", current_pp: 30 },
        ],
      }),
      null,
      null,
      null,
      null,
      null,
    ];
    const runner = createScriptRunnerStub({
      game_state: gameState,
      variables: { _selected_party_index: 0 },
    });

    expect(() => move_deletion(gameState, { runner })).toThrow(
      "MoveDeletion requires an explicit move selection."
    );
  });

  it("deletes the selected move from the chosen party mon", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", 158, {
        moves: [
          { name: "SCRATCH", current_pp: 35 },
          { name: "LEER", current_pp: 30 },
          { name: "RAGE", current_pp: 20 },
        ],
      }),
      null,
      null,
      null,
      null,
      null,
    ];
    const runner = createScriptRunnerStub({
      game_state: gameState,
      variables: { _selected_party_index: 0, _selected_move_index: 1 },
    });

    const result = move_deletion(gameState, { runner });

    expect(result).toBe("LEER");
    expect(runner.last_value).toBe("LEER");
    expect(runner.last_condition_result).toBe(true);
    expect(gameState.sram.party.pokemon[0]?.moves.map((move) => move.name)).toEqual([
      "SCRATCH",
      "RAGE",
    ]);
  });
});

describe("photo_studio", () => {
  it("treats printer failure as the ASM cancel path instead of saving a photo", () => {
    const gameState = createInitialGameState();
    gameState.sram.party.pokemon = [
      createTestPokemon("TOTODILE", 158, { nickname: "SNAP" }),
    ];
    (gameState.hram as typeof gameState.hram & { hPrinter?: number }).hPrinter = 1;
    const runner = createScriptRunnerStub({ game_state: gameState });
    const shownTexts: string[] = [];
    runner.event_manager.on("show_text", (event) => {
      shownTexts.push(String(event.data.text ?? ""));
    });

    const result = photo_studio(gameState, { runner, event_manager: runner.event_manager });

    expect(result).toBe(false);
    expect(gameState.sram.photo_album).toHaveLength(0);
    expect(gameState.wram.script_memory.wScriptVar).toBe(0);
    expect(runner.last_value).toBe(0);
    expect(runner.last_condition_result).toBe(false);
    expect(shownTexts.at(-1)).toContain("Oh, no picture?");
  });
});

describe("prof_oaks_pc_boot", () => {
  it("fills Oak rating buffers from seen and owned flags and records the chosen rating label", () => {
    const gameState = createInitialGameState();
    gameState.sram.pokedex_seen[0] = 0b0000_0111;
    gameState.sram.pokedex_seen[1] = 0b0000_1001;
    gameState.sram.pokedex_owned[0] = 0b0000_0111;
    const runner = createScriptRunnerStub({ variables: { _value: 99 } });

    const result = prof_oaks_pc_boot(gameState, { runner });

    expect(result).toBe(true);
    expect(runner.string_buffers.STRING_BUFFER_3).toBe("5");
    expect(runner.string_buffers.STRING_BUFFER_4).toBe("3");
    expect(runner.variables._value).toBe(99);
    expect(runner.variables._oak_seen_count).toBe(5);
    expect(runner.variables._oak_owned_count).toBe(3);
    expect(runner.variables._oak_rating_label).toBe("OakRating01");
    expect(runner.last_value).toBeUndefined();
    expect(runner.last_condition_result).toBe(true);
  });

  it("selects Oak ratings from owned flags instead of the caught set size", () => {
    const gameState = createInitialGameState();
    gameState.sram.pokedex_seen[0] = 0xff;
    gameState.sram.pokedex_seen[1] = 0x01;
    gameState.sram.pokedex_owned[0] = 0xff;
    gameState.sram.pokedex_owned[1] = 0x0f;
    gameState.sram.pokedex_caught = new Set([1]);
    const runner = createScriptRunnerStub();

    prof_oaks_pc_boot(gameState, { runner });

    expect(runner.string_buffers.STRING_BUFFER_3).toBe("9");
    expect(runner.string_buffers.STRING_BUFFER_4).toBe("12");
    expect(runner.variables._oak_rating_label).toBe("OakRating02");
  });
});
