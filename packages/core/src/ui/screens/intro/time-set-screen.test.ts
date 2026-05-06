import { createInitialGameState } from "@pokecrystal/core/core/state";
import { SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { KEYS } from "@pokecrystal/core/core/keycodes";
import { TimeSetScreen } from "./time-set-screen";
import { BootTextboxRenderer } from "./boot-textbox-renderer";

jest.mock("./boot-textbox-renderer", () => {
  return {
    BootTextboxRenderer: jest.fn().mockImplementation(() => ({
      drawTextBox: jest.fn(),
      drawWindow: jest.fn(),
      drawText: jest.fn(),
      drawPromptArrow: jest.fn(),
    })),
  };
});

const confirmEvent = { type: KEYS.KEYDOWN, key: KEYS.Z, code: "KeyZ" } as const;

describe("TimeSetScreen", () => {
  it("keeps yes/no cursor movement silent and only clicks on confirm", () => {
    const audioEngine = { playSound: jest.fn() } as any;
    const screen = new TimeSetScreen(createInitialGameState(), audioEngine, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    });

    for (let i = 0; i < 4; i += 1) {
      screen.handleInput(confirmEvent);
      screen.handleInput(confirmEvent);
    }
    for (let i = 0; i < 10; i += 1) {
      screen.update();
    }
    screen.handleInput(confirmEvent);

    audioEngine.playSound.mockClear();
    screen.handleInput({ type: KEYS.KEYDOWN, key: KEYS.DOWN, code: "ArrowDown" } as const);
    expect(audioEngine.playSound).not.toHaveBeenCalled();

    screen.handleInput(confirmEvent);
    expect(audioEngine.playSound).toHaveBeenCalledWith("menu_option");
  });

  it("uses ASM yes/no box coordinates during time confirmation prompts", () => {
    const screen = new TimeSetScreen(createInitialGameState(), null, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    });

    for (let i = 0; i < 4; i += 1) {
      screen.handleInput(confirmEvent);
      screen.handleInput(confirmEvent);
    }
    for (let i = 0; i < 10; i += 1) {
      screen.update();
    }
    screen.handleInput(confirmEvent);

    const ctx = {
      fillStyle: "#000000",
      fillRect: jest.fn(),
      canvas: { width: 160, height: 144 },
    } as unknown as CanvasRenderingContext2D;

    screen.draw(ctx);

    const rendererInstance = (BootTextboxRenderer as jest.Mock).mock.results.at(-1)?.value;
    expect(rendererInstance.drawWindow).toHaveBeenCalledWith(ctx, 14, 7, 6, 4);
  });

  it("renders the standard speech textbox for time prompts", () => {
    const screen = new TimeSetScreen(createInitialGameState(), null, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    });
    (screen as unknown as { phase: string }).phase = "set_hour";

    const ctx = {
      fillStyle: "#000000",
      fillRect: jest.fn(),
      canvas: { width: 160, height: 144 },
    } as unknown as CanvasRenderingContext2D;

    screen.draw(ctx);

    const rendererInstance = (BootTextboxRenderer as jest.Mock).mock.results.at(-1)?.value;
    expect(rendererInstance.drawTextBox).toHaveBeenCalledWith(
      ctx,
      "What time is it?",
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES
    );
  });

  it("uses ASM total box dimensions for the hour selector", () => {
    const screen = new TimeSetScreen(createInitialGameState(), null, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    }) as TimeSetScreen & { phase: string };
    screen.phase = "set_hour";

    const ctx = {
      fillStyle: "#000000",
      fillRect: jest.fn(),
      canvas: { width: 160, height: 144 },
    } as unknown as CanvasRenderingContext2D;

    screen.draw(ctx);

    const rendererInstance = (BootTextboxRenderer as jest.Mock).mock.results.at(-1)?.value;
    expect(rendererInstance.drawWindow).toHaveBeenCalledWith(ctx, 3, 7, 17, 4);
  });

  it("builds a text snapshot for the active time-setting prompt", () => {
    const screen = new TimeSetScreen(createInitialGameState(), null, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    }) as TimeSetScreen & {
      phase: string;
      hour: number;
    };
    screen.phase = "set_hour";
    screen.hour = 15;

    const snapshot = screen.getTextSnapshot();

    expect(snapshot.viewportTitle).toBe("Prompt");
    expect(snapshot.viewportLines).toEqual(["Prompt"]);
    expect(snapshot.infoTitle).toBe("Legend");
    expect(snapshot.infoLines).toEqual([
      "Use move up/down to adjust hour; press a to select; press b to back.",
    ]);
    expect(snapshot.dialogueLines).toEqual(["What time is it?"]);
    expect(snapshot.menuLines).toEqual(["DAY  3 o'clock"]);
    expect(snapshot.promptLines).toBeNull();
  });

  it("accepts direct player button-only inputs at the hour prompt", () => {
    const screen = new TimeSetScreen(createInitialGameState(), null, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    }) as TimeSetScreen & {
      phase: string;
      inputCooldown: number;
      hour: number;
    };
    screen.phase = "set_hour";
    screen.inputCooldown = 0;

    screen.handleInput({ type: "keydown", direction: "up", is_press: true } as any);
    expect(screen.hour).toBe(11);

    screen.handleInput({ type: "keydown", button: "a", is_press: true } as any);
    for (let i = 0; i < 100; i += 1) {
      screen.update();
    }

    expect(screen.getPhase()).toBe("hour_confirm");
    expect(screen.getTextSnapshot().dialogueLines).toEqual(["What?", "DAY 11 o'clock?"]);
  });

  it("can finish time setup from direct player button-only events", () => {
    const screen = new TimeSetScreen(createInitialGameState(), null, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    }) as TimeSetScreen & {
      phase: string;
      inputCooldown: number;
    };
    const pressA = (): void => {
      screen.handleInput({ type: "keydown", button: "a", is_press: true } as any);
    };
    const advance = (frames = 100): void => {
      for (let i = 0; i < frames; i += 1) {
        screen.update();
      }
    };

    screen.phase = "set_hour";
    screen.inputCooldown = 0;
    pressA();
    advance();
    expect(screen.getPhase()).toBe("hour_confirm");

    pressA();
    advance(10);
    expect(screen.getPhase()).toBe("set_minute");

    pressA();
    advance();
    expect(screen.getPhase()).toBe("minute_confirm");

    pressA();
    advance();
    expect(screen.getPhase()).toBe("final_reaction");

    pressA();
    pressA();
    expect(screen.isFinished()).toBe(true);
  });

  it("renders intro time confirmation as dialogue plus a real yes/no prompt", () => {
    const screen = new TimeSetScreen(createInitialGameState(), null, {
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    }) as TimeSetScreen & {
      phase: string;
      dialogue: { getVisibleText: () => string };
      yesNo: { result: () => boolean };
    };
    screen.phase = "minute_confirm";
    screen.dialogue = {
      getVisibleText: () => "Whoa!\n30 min.?",
    };
    screen.yesNo = {
      result: () => false,
    };

    const snapshot = screen.getTextSnapshot();

    expect(snapshot.viewportLines).toEqual(["Prompt"]);
    expect(snapshot.infoLines).toEqual([
      "Use move up/down to choose YES/NO; press a to select; press b to cancel.",
    ]);
    expect(snapshot.dialogueLines).toEqual(["Whoa!", "30 min.?"]);
    expect(snapshot.menuLines).toBeNull();
    expect(snapshot.promptLines).toEqual(["   YES", "▶ NO"]);
  });
});
