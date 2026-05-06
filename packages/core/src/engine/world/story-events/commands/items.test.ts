import { createInitialGameState } from "@pokecrystal/core/core/state";
import { setPokedexFlag } from "@pokecrystal/core/core/pokedex";
import { createScriptRunnerStub } from "@pokecrystal/core/engine/world/story-events/test-utils";
import { GetItemNameCommand, GetTrainerNameCommand, HiddenItemCommand, ReadVarCommand } from "./items";

describe("ReadVarCommand", () => {
  it("counts pokedex seen from flag bytes", () => {
    const gameState = createInitialGameState();
    setPokedexFlag(gameState, 152, "seen");
    setPokedexFlag(gameState, 158, "seen");
    const runner = createScriptRunnerStub({ game_state: gameState });
    const command = new ReadVarCommand("VAR_DEXSEEN");
    command.runner = runner;

    command.execute(gameState, runner.event_manager, runner.overworld);

    expect(runner.last_value).toBe(2);
  });

  it("counts pokedex owned from flag bytes", () => {
    const gameState = createInitialGameState();
    setPokedexFlag(gameState, 152, "owned");
    setPokedexFlag(gameState, 158, "owned");
    setPokedexFlag(gameState, 160, "owned");
    const runner = createScriptRunnerStub({ game_state: gameState });
    const command = new ReadVarCommand("VAR_DEXCAUGHT");
    command.runner = runner;

    command.execute(gameState, runner.event_manager, runner.overworld);

    expect(runner.last_value).toBe(3);
  });

  it("counts VAR_BADGES in ASM Johto/Kanto order", () => {
    const gameState = createInitialGameState();
    gameState.sram.badges.johto[0] = true;
    gameState.sram.badges.johto[7] = true;
    gameState.sram.badges.kanto[3] = true;
    const runner = createScriptRunnerStub({ game_state: gameState });
    const command = new ReadVarCommand("VAR_BADGES");
    command.runner = runner;

    command.execute(gameState, runner.event_manager, runner.overworld);

    expect(runner.last_value).toBe(3);
  });

  it("throws for non-ASM badge bank lengths", () => {
    const gameState = createInitialGameState();
    gameState.sram.badges.kanto = [true, false] as boolean[];
    const runner = createScriptRunnerStub({ game_state: gameState });
    const command = new ReadVarCommand("VAR_BADGES");
    command.runner = runner;

    expect(() => command.execute(gameState, runner.event_manager, runner.overworld)).toThrow("must contain exactly 8");
  });
});

describe("HiddenItemCommand", () => {
  it("stores the hidden item name in STRING_BUFFER_3 before the bag-capacity branch", () => {
    const gameState = createInitialGameState();
    gameState.sram.player_name = "KRIS";
    const eventManager = createScriptRunnerStub({ game_state: gameState }).event_manager;
    const showHandler = jest.fn();
    eventManager.on("show_text", showHandler);
    const runner = createScriptRunnerStub({
      game_state: gameState,
      event_manager: eventManager,
      overworld: {
        item_system: {
          addItem: jest.fn(() => false),
          getDisplayName: jest.fn(() => "ANTIDOTE"),
        },
      } as any,
    });
    const command = new HiddenItemCommand("ANTIDOTE", "EVENT_GOT_HIDDEN_ANTIDOTE");
    command.runner = runner;

    command.execute(gameState, eventManager, runner.overworld);

    expect(runner.string_buffers?.STRING_BUFFER_3).toBe("ANTIDOTE");
    expect(showHandler.mock.calls[0]?.[0]?.data?.text).toBe("KRIS found\nANTIDOTE!");
    expect(showHandler.mock.calls[1]?.[0]?.data?.text).toBe("But KRIS has\nno space left...");
  });
});

describe("GetItemNameCommand", () => {
  it("throws instead of prettifying an item id when item data is missing", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({
      game_state: gameState,
      overworld: {
        data_loader: {
          get_item: jest.fn(() => null),
        },
      } as any,
    });
    const command = new GetItemNameCommand("STRING_BUFFER_3", "SILVER_LEAF");
    command.runner = runner;

    expect(() =>
      command.execute(gameState, runner.event_manager, runner.overworld),
    ).toThrow("Missing ASM item name for 'SILVER_LEAF'.");
  });
});

describe("GetTrainerNameCommand", () => {
  it("uses trainer data instead of fabricating a prettified class/id string", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({
      game_state: gameState,
      overworld: {
        data_loader: {
          get_trainer: jest.fn((trainerId: string) =>
            trainerId === "DANA1"
              ? ({
                  name: "Dana",
                } as any)
              : null,
          ),
        },
      } as any,
    });
    const command = new GetTrainerNameCommand("STRING_BUFFER_3", "LASS", "DANA1");
    command.runner = runner;

    command.execute(gameState, runner.event_manager, runner.overworld);

    expect(runner.string_buffers?.STRING_BUFFER_3).toBe("Dana");
  });

  it("throws instead of synthesizing a trainer name from class/id tokens", () => {
    const gameState = createInitialGameState();
    const runner = createScriptRunnerStub({
      game_state: gameState,
      overworld: {
        data_loader: {
          get_trainer: jest.fn(() => null),
        },
      } as any,
    });
    const command = new GetTrainerNameCommand("STRING_BUFFER_3", "LASS", "DANA1");
    command.runner = runner;

    expect(() =>
      command.execute(gameState, runner.event_manager, runner.overworld),
    ).toThrow("Missing ASM trainer name for 'LASS/DANA1'.");
  });
});
