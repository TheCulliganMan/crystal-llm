import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { Surface } from "@pokecrystal/core/ui/surface";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { GameButton } from "@pokecrystal/core/input/controls";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { NameEntryScreen } from "./name-entry-screen";

class NameEntryUiStub extends BaseUI {
  public readonly font = { renderText: jest.fn() };

  constructor() {
    super(160, 144, 1);
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  update(): void {}
}

class TextOnlyNameEntryUiStub extends NameEntryUiStub {
  public readonly renderSnapshot = jest.fn();
}

class ScaledNameEntryUiStub extends BaseUI {
  public readonly font = { renderText: jest.fn() };

  constructor() {
    super(400, 300, 1);
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  update(): void {}
}

const makeSolidSurface = (
  width: number,
  height: number,
  color: [number, number, number, number]
): InstanceType<typeof gameEngine.Surface> => {
  const surface = new gameEngine.Surface(width, height);
  surface.fill(color);
  return surface;
};

const installNamingAssets = (): jest.SpyInstance<ReturnType<typeof gameEngine.image.loadSync>, Parameters<typeof gameEngine.image.loadSync>> => {
  const fontSurface = new gameEngine.Surface(TILE_SIZE * 32, TILE_SIZE * 8);
  fontSurface.fill([255, 255, 255, 255]);
  fontSurface.fill([0, 0, 0, 255], { x: 0, y: 0, width: TILE_SIZE * 32, height: TILE_SIZE * 8 });

  const cursorSurface = new gameEngine.Surface(TILE_SIZE, TILE_SIZE * 2);
  cursorSurface.fill([0, 0, 255, 255], { x: 0, y: 0, width: TILE_SIZE, height: TILE_SIZE });
  cursorSurface.fill([255, 0, 255, 255], { x: 0, y: TILE_SIZE, width: TILE_SIZE, height: TILE_SIZE });

  const borderSurface = makeSolidSurface(TILE_SIZE, TILE_SIZE, [255, 255, 255, 255]);
  borderSurface.set_at([1, 1], [85, 85, 85, 255]);
  const underlineSurface = makeSolidSurface(TILE_SIZE, TILE_SIZE, [200, 200, 0, 255]);
  const middleLineSurface = makeSolidSurface(TILE_SIZE, TILE_SIZE, [0, 200, 200, 255]);

  return jest.spyOn(gameEngine.image, "loadSync").mockImplementation((path: string) => {
    if (path.includes("gfx/font/font.png")) {
      return fontSurface;
    }
    if (path.includes("gfx/naming_screen/cursor.png")) {
      return cursorSurface;
    }
    if (path.includes("gfx/naming_screen/border.png")) {
      return borderSurface;
    }
    if (path.includes("gfx/naming_screen/underline.png")) {
      return underlineSurface;
    }
    if (path.includes("gfx/naming_screen/middle_line.png")) {
      return middleLineSurface;
    }
    return null;
  });
};

const pressA = (screen: NameEntryScreen): void => {
  screen.handleInput(new gameEngine.event.Event("keydown", { button: GameButton.A, is_press: true }));
};

const pressDirection = (screen: NameEntryScreen, direction: "up" | "down" | "left" | "right"): void => {
  screen.handleInput(new gameEngine.event.Event("keydown", { direction, is_press: true }));
};

const releaseDirection = (screen: NameEntryScreen, direction: "up" | "down" | "left" | "right"): void => {
  screen.handleInput(new gameEngine.event.Event("keyup", { direction, is_press: false }));
};

const tapDirection = (screen: NameEntryScreen, direction: "up" | "down" | "left" | "right"): void => {
  pressDirection(screen, direction);
  releaseDirection(screen, direction);
};

describe("NameEntryScreen", () => {
  let loadSyncMock: jest.SpyInstance;

  beforeEach(() => {
    loadSyncMock = installNamingAssets();
  });

  afterEach(() => {
    loadSyncMock.mockRestore();
  });

  it("treats Backspace as delete", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");

    screen.handleInput(new gameEngine.event.Event("keydown", { text: "A" }));
    expect(screen.name).toBe("A");

    screen.handleInput(new gameEngine.event.Event("keydown", { key: "Backspace" }));
    expect(screen.name).toBe("");
  });

  it("preserves typed text case for name entry", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");

    for (const char of "RYAN") {
      screen.handleInput(new gameEngine.event.Event("keydown", { text: char }));
    }
    expect(screen.name).toBe("RYAN");

    screen.reset();
    for (const char of "thor") {
      screen.handleInput(new gameEngine.event.Event("keydown", { text: char }));
    }
    expect(screen.name).toBe("thor");
  });

  it("accepts type_text-style space input as a real naming-screen glyph", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");

    screen.handleInput(new gameEngine.event.Event("keydown", { text: "A" }));
    screen.handleInput(new gameEngine.event.Event("keydown", { text: " " }));
    screen.handleInput(new gameEngine.event.Event("keydown", { text: "B" }));

    expect(screen.name).toBe("A B");
  });

  it("keeps naming-screen input silent like the asm naming screen", () => {
    const ui = new NameEntryUiStub();
    const audioEngine = { playSound: jest.fn() } as any;
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?", audioEngine);

    screen.handleInput(new gameEngine.event.Event("keydown", { key: "ArrowRight" }));
    screen.handleInput(new gameEngine.event.Event("keydown", { text: "A" }));
    screen.handleInput(new gameEngine.event.Event("keydown", { key: "Backspace" }));

    expect(audioEngine.playSound).not.toHaveBeenCalled();
  });

  it("moves horizontally from code-only direction events", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");

    screen.handleInput(new gameEngine.event.Event("keydown", { code: "ArrowRight" }));
    expect(screen.cursorPos).toEqual([1, 0]);

    screen.handleInput(new gameEngine.event.Event("keydown", { code: "ArrowLeft" }));
    expect(screen.cursorPos).toEqual([0, 0]);
  });

  it("selects every upper-case keyboard letter with A", () => {
    const ui = new NameEntryUiStub();
    const positions = [
      ["A", 0, 0], ["B", 1, 0], ["C", 2, 0], ["D", 3, 0], ["E", 4, 0],
      ["F", 5, 0], ["G", 6, 0], ["H", 7, 0], ["I", 8, 0],
      ["J", 0, 1], ["K", 1, 1], ["L", 2, 1], ["M", 3, 1], ["N", 4, 1],
      ["O", 5, 1], ["P", 6, 1], ["Q", 7, 1], ["R", 8, 1],
      ["S", 0, 2], ["T", 1, 2], ["U", 2, 2], ["V", 3, 2], ["W", 4, 2],
      ["X", 5, 2], ["Y", 6, 2], ["Z", 7, 2],
    ] as const;

    for (const [letter, column, row] of positions) {
      const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");
      screen.cursorPos = [column, row];
      pressA(screen);
      expect(screen.name).toBe(letter);
    }
  });

  it("selects every lower-case keyboard letter with A", () => {
    const ui = new NameEntryUiStub();
    const positions = [
      ["a", 0, 0], ["b", 1, 0], ["c", 2, 0], ["d", 3, 0], ["e", 4, 0],
      ["f", 5, 0], ["g", 6, 0], ["h", 7, 0], ["i", 8, 0],
      ["j", 0, 1], ["k", 1, 1], ["l", 2, 1], ["m", 3, 1], ["n", 4, 1],
      ["o", 5, 1], ["p", 6, 1], ["q", 7, 1], ["r", 8, 1],
      ["s", 0, 2], ["t", 1, 2], ["u", 2, 2], ["v", 3, 2], ["w", 4, 2],
      ["x", 5, 2], ["y", 6, 2], ["z", 7, 2],
    ] as const;

    for (const [letter, column, row] of positions) {
      const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");
      screen.cursorPos = [0, NameEntryScreen.BOTTOM_ROW_INDEX];
      pressA(screen);
      screen.cursorPos = [column, row];
      pressA(screen);
      expect(screen.name).toBe(letter);
    }
  });

  it("shows why lower-case r is blocked at max length, then accepts it after delete", () => {
    const ui = new TextOnlyNameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");
    screen.reset({ maxNameLength: 1 });

    pressA(screen);
    screen.cursorPos = [0, NameEntryScreen.BOTTOM_ROW_INDEX];
    pressA(screen);
    screen.cursorPos = [8, 1];
    screen.draw();

    let info = ui.renderSnapshot.mock.calls.at(-1)?.[1] as string[];
    expect(info).toContain("SELECTED: r");
    expect(info).toContain("STATUS: full - delete before adding");

    pressA(screen);
    expect(screen.name).toBe("A");

    screen.handleInput(new gameEngine.event.Event("keydown", { button: GameButton.B, is_press: true }));
    pressA(screen);

    expect(screen.name).toBe("r");
  });

  it("supports every bottom-row command", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");

    screen.cursorPos = [0, NameEntryScreen.BOTTOM_ROW_INDEX];
    pressA(screen);
    expect(screen.case).toBe("lower");

    screen.cursorPos = [0, NameEntryScreen.BOTTOM_ROW_INDEX];
    pressA(screen);
    expect(screen.case).toBe("upper");

    screen.cursorPos = [0, 0];
    pressA(screen);
    expect(screen.name).toBe("A");

    screen.cursorPos = [3, NameEntryScreen.BOTTOM_ROW_INDEX];
    pressA(screen);
    expect(screen.name).toBe("");

    screen.cursorPos = [0, 0];
    pressA(screen);
    screen.cursorPos = [6, NameEntryScreen.BOTTOM_ROW_INDEX];
    pressA(screen);
    expect(screen.finished).toBe(true);
  });

  it("confirms blank names from END so default-name flows can continue", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");

    screen.cursorPos = [6, NameEntryScreen.BOTTOM_ROW_INDEX];
    pressA(screen);

    expect(screen.name).toBe("");
    expect(screen.finished).toBe(true);
  });

  it("confirms after keyboard navigation selects END", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");

    pressDirection(screen, "down");
    pressDirection(screen, "down");
    pressDirection(screen, "down");
    pressDirection(screen, "down");
    pressDirection(screen, "right");
    pressDirection(screen, "right");
    expect(screen.cursorPos).toEqual([6, NameEntryScreen.BOTTOM_ROW_INDEX]);

    pressA(screen);

    expect(screen.finished).toBe(true);
  });

  it("prioritizes an explicit A button over the keyboard a movement alias", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");
    screen.fillName("TOTODILE");
    screen.cursorPos = [8, NameEntryScreen.BOTTOM_ROW_INDEX];

    screen.handleInput(
      new gameEngine.event.Event("keydown", {
        key: "a",
        code: "KeyA",
        button: GameButton.A,
        direction: null,
        is_press: true,
      })
    );

    expect(screen.finished).toBe(true);
  });

  it("applies CLI typed-letter movement shortcuts across the real naming screen", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");

    tapDirection(screen, "right");
    tapDirection(screen, "right");
    tapDirection(screen, "right");
    pressA(screen);

    expect(screen.name).toBe("D");
    expect(screen.cursorPos).toEqual([3, 0]);
  });

  it("accepts repeated CLI typed-letter shortcut directions", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");

    tapDirection(screen, "right");
    tapDirection(screen, "right");
    pressA(screen);
    tapDirection(screen, "right");
    tapDirection(screen, "right");
    pressA(screen);

    expect(screen.name).toBe("CE");
  });

  it("moves to END on Start without confirming", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");

    screen.handleInput(new gameEngine.event.Event("keydown", { button: GameButton.Start, is_press: true }));

    expect(screen.name).toBe("");
    expect(screen.finished).toBe(false);
    expect(screen.cursorPos).toEqual([8, NameEntryScreen.BOTTOM_ROW_INDEX]);
  });

  it("moves to END at max length and only confirms from END", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");
    screen.reset({ maxNameLength: 1 });

    pressA(screen);

    expect(screen.name).toBe("A");
    expect(screen.finished).toBe(false);
    expect(screen.cursorPos).toEqual([8, NameEntryScreen.BOTTOM_ROW_INDEX]);

    screen.cursorPos = [1, 0];
    pressA(screen);

    expect(screen.name).toBe("A");
    expect(screen.finished).toBe(false);

    screen.cursorPos = [6, NameEntryScreen.BOTTOM_ROW_INDEX];
    pressA(screen);

    expect(screen.finished).toBe(true);
  });

  it("deletes with B instead of confirming a full name", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");
    screen.reset({ maxNameLength: 1 });
    pressA(screen);

    screen.handleInput(new gameEngine.event.Event("keydown", { button: GameButton.B, is_press: true }));

    expect(screen.name).toBe("");
    expect(screen.finished).toBe(false);
  });

  it("loads font and special tiles for the naming screen", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");
    const tileset = (screen as unknown as { tileset: { fontTiles: Record<number, InstanceType<typeof gameEngine.Surface>>; getTileSurface: (index: number) => InstanceType<typeof gameEngine.Surface>; specialTiles: Record<string, InstanceType<typeof gameEngine.Surface>>; } }).tileset;

    expect(loadSyncMock).toHaveBeenCalledWith(expect.stringContaining("gfx/font/font.png"));
    expect(loadSyncMock).toHaveBeenCalledWith(expect.stringContaining("gfx/naming_screen/cursor.png"));
    expect(loadSyncMock).toHaveBeenCalledWith(expect.stringContaining("gfx/naming_screen/border.png"));
    expect(loadSyncMock).toHaveBeenCalledWith(expect.stringContaining("gfx/naming_screen/underline.png"));
    expect(loadSyncMock).toHaveBeenCalledWith(expect.stringContaining("gfx/naming_screen/middle_line.png"));

    expect(tileset.fontTiles[0]).toBeDefined();
    expect(tileset.getTileSurface(0x80)).toBe(tileset.fontTiles[0]);
    expect(tileset.getTileSurface(NameEntryScreen.NAMINGSCREEN_BORDER)).toBeDefined();
    expect(tileset.getTileSurface(NameEntryScreen.NAMINGSCREEN_UNDERLINE)).toBeDefined();
    expect(tileset.getTileSurface(NameEntryScreen.NAMINGSCREEN_MIDDLELINE)).toBeDefined();
    expect(tileset.specialTiles.cursor_0).toBeDefined();
    expect(tileset.specialTiles.cursor_1).toBeDefined();
  });

  it("colors the border with the ASM diploma background palette", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");

    screen.draw();

    expect(ui.screen.get_at([0, 0])).toEqual([222, 255, 222, 255]);
    expect(ui.screen.get_at([1, 1])).toEqual([107, 107, 107, 255]);
  });

  it("accepts numeric keycodes for text entry", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");

    screen.handleInput(new gameEngine.event.Event("keydown", { key: 102 }));
    expect(screen.name).toBe("F");
  });

  it("can prefill a name up to the max length", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");
    screen.reset({ maxNameLength: 3 });
    screen.fillName("PIKACHU");
    expect(screen.name).toBe("PIK");
  });

  it("supports the Oak intro player-name prompt with the standard boot length", () => {
    const ui = new NameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");

    screen.reset({ prompt: "YOUR NAME?", maxNameLength: 7 });
    screen.fillName("KRYSTAL");

    expect(screen.name).toBe("KRYSTAL");
  });

  it("clears the complete host target before centering one integer-scaled LCD", () => {
    const ui = new ScaledNameEntryUiStub();
    const screen = new NameEntryScreen(ui, "YOUR NAME?");

    // Prove draw owns the whole target rather than accidentally succeeding
    // because the fixture began blank like an unloaded overworld.
    ui.screen.fill([255, 0, 0, 255]);
    screen.draw();

    // 160x144 scales to 320x288 in a 400x300 host and is centered at (40, 6).
    expect(ui.screen.get_at([0, 0])).toEqual([255, 255, 255, 255]);
    expect(ui.screen.get_at([39, 150])).toEqual([255, 255, 255, 255]);
    expect(ui.screen.get_at([40, 6])).toEqual([222, 255, 222, 255]);
    expect(ui.screen.get_at([359, 293])).toEqual([222, 255, 222, 255]);
    expect(ui.screen.get_at([360, 294])).toEqual([255, 255, 255, 255]);
  });

  it("skips pixel tilemap composition when the renderer is pure text", () => {
    const ui = new TextOnlyNameEntryUiStub();
    const blitSpy = jest.spyOn(ui.screen, "blit");
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");

    screen.draw();

    expect(blitSpy).not.toHaveBeenCalled();
    expect(ui.renderSnapshot).toHaveBeenCalled();
  });

  it("reports the selected name-entry cell in text snapshots", () => {
    const ui = new TextOnlyNameEntryUiStub();
    const screen = new NameEntryScreen(ui, "RIVAL'S NAME?");

    screen.draw();
    let info = ui.renderSnapshot.mock.calls.at(-1)?.[1] as string[];
    expect(info).toContain("Use move up/down/left/right to move the cursor; press a to select.");
    expect(info).toContain("Use press b to delete, press start to choose END, or type_text for letters.");
    expect(info).toContain("SELECTED: A");

    tapDirection(screen, "up");
    tapDirection(screen, "right");
    screen.draw();
    info = ui.renderSnapshot.mock.calls.at(-1)?.[1] as string[];
    expect(info).toContain("SELECTED: DEL");

    tapDirection(screen, "right");
    screen.draw();
    info = ui.renderSnapshot.mock.calls.at(-1)?.[1] as string[];
    expect(info).toContain("SELECTED: END");
  });
});
