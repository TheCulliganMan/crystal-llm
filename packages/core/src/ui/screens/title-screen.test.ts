/** @jest-environment jsdom */
import { TitleScreen } from "./title-screen";

type MockTile = [number, number, number, number][][];

const makeTransparentTile = (): MockTile =>
  Array.from({ length: 8 }, () =>
    Array.from({ length: 8 }, () => [0, 0, 0, 0] as [number, number, number, number])
  );

const makeZeroIndices = (): number[][] =>
  Array.from({ length: 8 }, () => Array(8).fill(0));

const makeMockGraphics = () => {
  const crystalTop = makeTransparentTile();
  crystalTop[0][0] = [200, 100, 50, 255];
  const crystalBottom = makeTransparentTile();

  const crystalTopIndices = makeZeroIndices();
  crystalTopIndices[0][0] = 1;
  const crystalBottomIndices = makeZeroIndices();

  return {
    getTile: jest.fn((graphicName: string, tileIndex: number) => {
      if (graphicName === "crystal") {
        return (tileIndex & 0xff) === 0 ? crystalTop : crystalBottom;
      }
      return makeTransparentTile();
    }),
    getTileIndices: jest.fn((graphicName: string, tileIndex: number) => {
      if (graphicName === "crystal") {
        return (tileIndex & 0xff) === 0 ? crystalTopIndices : crystalBottomIndices;
      }
      return makeZeroIndices();
    }),
  };
};

const makeMockAudio = () =>
  ({
    channelsOff: jest.fn(),
    playSound: jest.fn(),
    playMusic: jest.fn(),
    fadeOutMusic: jest.fn(),
    stopMusic: jest.fn(),
  }) as any;

describe("title-screen crystal priority", () => {
  it("hides crystal pixels behind non-zero BG/WIN color indices", () => {
    const graphics = makeMockGraphics();
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());
    (screen as any).crystalSprites = [[8, 16, 0]];

    const fillRect = jest.fn();
    const ctx = { fillStyle: "", fillRect } as unknown as CanvasRenderingContext2D;
    const priorityMap = Array.from({ length: 144 }, () => Array(160).fill(0));

    (screen as any)._drawCrystalTileWithPriority(
      ctx,
      graphics.getTile("crystal", 0, 0),
      graphics.getTileIndices("crystal", 0),
      0,
      0,
      priorityMap
    );
    expect(fillRect).toHaveBeenCalledWith(0, 0, 1, 1);

    fillRect.mockClear();
    priorityMap[0][0] = 2;
    (screen as any)._drawCrystalTileWithPriority(
      ctx,
      graphics.getTile("crystal", 0, 0),
      graphics.getTileIndices("crystal", 0),
      0,
      0,
      priorityMap
    );
    expect(fillRect).not.toHaveBeenCalled();
  });

  it("renders crystal pixels into a cached transparent layer before compositing", () => {
    const graphics = makeMockGraphics();
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());

    const crystalFillRect = jest.fn();
    screen.crystalContext = {
      clearRect: jest.fn(),
      fillRect: crystalFillRect,
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;

    const ctx = {
      fillStyle: "",
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      save: jest.fn(),
      restore: jest.fn(),
      setTransform: jest.fn(),
      globalAlpha: 1,
      globalCompositeOperation: "source-over",
    } as unknown as CanvasRenderingContext2D;

    screen.crystalSprites = [[8, 16, 0]];
    screen.draw(ctx);

    expect(crystalFillRect).toHaveBeenCalledWith(0, 0, 1, 1);
    expect(ctx.drawImage).toHaveBeenCalledWith(screen.crystalCanvas, 0, 0);
  });

  it("reuses offscreen canvases across repeated draws", () => {
    const graphics = makeMockGraphics();
    const originalCreateElement = document.createElement.bind(document);
    const makeCanvas = () => {
      const mockCanvasContext = {
        clearRect: jest.fn(),
        fillRect: jest.fn(),
        drawImage: jest.fn(),
      } as unknown as CanvasRenderingContext2D;
      return {
        width: 0,
        height: 0,
        getContext: jest.fn(() => mockCanvasContext),
      } as unknown as HTMLCanvasElement;
    };
    const createElementSpy = jest.spyOn(document, "createElement").mockImplementation((
      tagName: string
    ): HTMLElement => {
      if (tagName === "canvas") {
        return makeCanvas() as unknown as HTMLElement;
      }
      return originalCreateElement(tagName) as HTMLElement;
    });

    const screen = new (TitleScreen as any)(graphics, makeMockAudio());
    createElementSpy.mockClear();

    const ctx = {
      save: jest.fn(),
      restore: jest.fn(),
      setTransform: jest.fn(),
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      globalAlpha: 1,
      globalCompositeOperation: "source-over",
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;

    screen.draw(ctx);
    screen.draw(ctx);

    expect(createElementSpy).not.toHaveBeenCalled();

    createElementSpy.mockRestore();
  });

  it("reuses prerendered background and window layers across repeated draws", () => {
    const graphics = makeMockGraphics();
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());
    const renderBackgroundSpy = jest.spyOn(screen, "_renderBackground");
    const renderWindowSpy = jest.spyOn(screen, "_renderWindow");
    const ensureScrolledSpy = jest.spyOn(screen, "_ensureScrolledBackgroundLayerRendered");
    const ensureCrystalSpy = jest.spyOn(screen, "_ensureCrystalLayerRendered");
    const ctx = {
      save: jest.fn(),
      restore: jest.fn(),
      setTransform: jest.fn(),
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      globalAlpha: 1,
      globalCompositeOperation: "source-over",
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;

    screen.draw(ctx);
    screen.draw(ctx);

    expect(renderBackgroundSpy).toHaveBeenCalledTimes(1);
    expect(renderWindowSpy).toHaveBeenCalledTimes(1);
    expect(ensureScrolledSpy).toHaveBeenCalledTimes(2);
    expect(ensureCrystalSpy).toHaveBeenCalledTimes(2);

    screen.suicuneAnimationTimer = 7;
    screen.update();
    screen.draw(ctx);
    expect(renderBackgroundSpy).toHaveBeenCalledTimes(2);
    expect(renderWindowSpy).toHaveBeenCalledTimes(1);

    screen.registers.setWy(16);
    screen.draw(ctx);
    expect(renderWindowSpy).toHaveBeenCalledTimes(2);
  });

  it("caches the background priority map until title graphics state changes", () => {
    const graphics = makeMockGraphics();
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());
    const buildPriorityMapSpy = jest.spyOn(screen, "_buildBackgroundPriorityMap");
    const ctx = {
      save: jest.fn(),
      restore: jest.fn(),
      setTransform: jest.fn(),
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      globalAlpha: 1,
      globalCompositeOperation: "source-over",
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;

    screen.draw(ctx);
    screen.draw(ctx);
    expect(buildPriorityMapSpy).toHaveBeenCalledTimes(1);

    screen.suicuneAnimationTimer = 7;
    screen.update();
    screen.draw(ctx);
    expect(buildPriorityMapSpy).toHaveBeenCalledTimes(2);
  });

  it("builds crystal priority from the scrolled title background coordinates", () => {
    const graphics = makeMockGraphics();
    graphics.getTileIndices.mockImplementation((graphicName: string, tileIndex: number) => {
      if (graphicName === "logo" && (tileIndex & 0xff) === 0x8e) {
        return Array.from({ length: 8 }, () => Array(8).fill(2));
      }
      if (graphicName === "crystal") {
        return (tileIndex & 0xff) === 0 ? makeZeroIndices() : makeZeroIndices();
      }
      return makeZeroIndices();
    });
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());

    screen.registers.setScx(112);
    screen.lineScrollBuffer.setActive(true);
    screen.lineScrollBuffer.setUniform(3 * 8 - 8, 8, 112);

    const priorityMap = screen._buildBackgroundPriorityMap();

    expect(priorityMap[3 * 8 - 8][0]).toBe(2);
    expect(priorityMap[3 * 8 - 8][112]).toBe(0);
  });

  it("invalidates the crystal priority map when the entrance line scroll advances", () => {
    const screen = new (TitleScreen as any)(makeMockGraphics(), makeMockAudio());
    const buildPriorityMapSpy = jest.spyOn(screen, "_buildBackgroundPriorityMap");
    const ctx = {
      save: jest.fn(),
      restore: jest.fn(),
      setTransform: jest.fn(),
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      globalAlpha: 1,
      globalCompositeOperation: "source-over",
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;

    screen.draw(ctx);
    screen.update();
    screen.draw(ctx);

    expect(buildPriorityMapSpy).toHaveBeenCalledTimes(2);
  });

  it("positions the version window using WY/WX like the ASM title screen", () => {
    const graphics = makeMockGraphics();
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());

    const drawTileSpy = jest.spyOn(screen, "_drawTile");
    screen.registers.setWy(16);
    screen.registers.setWx(15);

    const windowCtx = {
      clearRect: jest.fn(),
    } as unknown as CanvasRenderingContext2D;
    screen._renderWindow(windowCtx);

    expect(windowCtx.clearRect).toHaveBeenCalledWith(0, 0, 160, 144);
    expect(drawTileSpy).toHaveBeenCalled();
    const firstCall = drawTileSpy.mock.calls[0];
    expect(firstCall[2]).toBe(32);
    expect(firstCall[3]).toBe(16);
  });

  it("applies SCY=8 to the ASM logo and Suicune tilemap anchors", () => {
    const graphics = makeMockGraphics();
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());
    const drawTileSpy = jest.spyOn(screen, "_drawTile");

    const backgroundCtx = {
      clearRect: jest.fn(),
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;

    screen._renderBackground(backgroundCtx);

    const suicuneCall = drawTileSpy.mock.calls[0];
    expect(suicuneCall[2]).toBe(6 * 8);   // column 6 → pixel 48
    expect(suicuneCall[3]).toBe(12 * 8 - 8); // hlcoord row 12 minus SCY

    const firstLogoCall = drawTileSpy.mock.calls[6 * 8];
    expect(firstLogoCall[2]).toBe(0);
    expect(firstLogoCall[3]).toBe(3 * 8 - 8); // hlcoord row 3 minus SCY
  });

  it("precomposites SCY into the screen-space background before line scrolling", () => {
    const graphics = makeMockGraphics();
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());
    screen.registers.setScy(8);

    const scrolledDrawImage = jest.fn();
    screen.scrolledBackgroundContext = {
      clearRect: jest.fn(),
      drawImage: scrolledDrawImage,
    } as unknown as CanvasRenderingContext2D;
    screen.backgroundContext = {
      clearRect: jest.fn(),
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;

    screen._ensureScrolledBackgroundLayerRendered();

    const firstBackgroundBlit = scrolledDrawImage.mock.calls[0];
    // SCY was already applied when the BG tiles were placed into this screen-space layer.
    expect(firstBackgroundBlit[2]).toBe(0);
    expect(firstBackgroundBlit[6]).toBe(0);
  });

  it("clears offscreen layers when a dirty layer is rerendered", () => {
    const graphics = makeMockGraphics();
    const screen = new (TitleScreen as any)(graphics, makeMockAudio());
    const backgroundClearSpy = jest.fn();
    const windowClearSpy = jest.fn();
    const backgroundContext = {
      clearRect: backgroundClearSpy,
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;
    const scrolledBackgroundContext = {
      clearRect: jest.fn(),
      drawImage: jest.fn(),
    } as unknown as CanvasRenderingContext2D;
    const windowContext = {
      clearRect: windowClearSpy,
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;
    screen.backgroundContext = backgroundContext;
    screen.scrolledBackgroundContext = scrolledBackgroundContext;
    screen.windowContext = windowContext;

    const ctx = {
      save: jest.fn(),
      restore: jest.fn(),
      setTransform: jest.fn(),
      fillRect: jest.fn(),
      drawImage: jest.fn(),
      globalAlpha: 1,
      globalCompositeOperation: "source-over",
      fillStyle: "",
    } as unknown as CanvasRenderingContext2D;

    screen.draw(ctx);
    screen.draw(ctx);

    expect(backgroundClearSpy).toHaveBeenCalledWith(0, 0, 160, 144);
    expect(windowClearSpy).toHaveBeenCalledWith(0, 0, 160, 144);
    expect(backgroundClearSpy).toHaveBeenCalledTimes(1);
    expect(windowClearSpy).toHaveBeenCalledTimes(1);

    screen.suicuneAnimationTimer = 7;
    screen.update();
    screen.draw(ctx);
    expect(backgroundClearSpy).toHaveBeenCalledTimes(2);
    expect(windowClearSpy).toHaveBeenCalledTimes(1);
  });
});

describe("title-screen input gating", () => {
  it("starts title music with an explicit title role after the entrance completes", () => {
    const audio = makeMockAudio();
    const screen = new (TitleScreen as any)(makeMockGraphics(), audio);

    screen.registers.setScx(0);
    screen.update();

    expect(audio.playMusic).toHaveBeenCalledWith("MUSIC_TITLE", "title");
  });

  it("does not treat a held A press from before MAIN as a fresh title-screen confirm", () => {
    const screen = new (TitleScreen as any)(makeMockGraphics(), makeMockAudio());

    screen.handleInput({ key: "a" } as KeyboardEvent, true);
    expect(screen.popAction()).toBeNull();

    screen.registers.setScx(0);
    screen.update();
    expect(screen.popAction()).toBeNull();

    screen.update();
    expect(screen.popAction()).toBeNull();

    screen.handleInput({ key: "a" } as KeyboardEvent, false);
    expect(screen.popAction()).toBeNull();

    screen.handleInput({ key: "a" } as KeyboardEvent, true);
    screen.update();
    expect(screen.popAction()).toBe("main_menu");
  });

  it("queues main menu on a fresh A press without playing a menu click", () => {
    const audio = makeMockAudio();
    const screen = new (TitleScreen as any)(makeMockGraphics(), audio);

    screen.registers.setScx(0);
    screen.update();
    screen.update();
    audio.playSound.mockClear();

    screen.handleInput({ key: "a" } as KeyboardEvent, true);
    screen.update();

    expect(screen.popAction()).toBe("main_menu");
    expect(audio.playSound).not.toHaveBeenCalled();
  });

  it("queues main menu from direct MCP/TUI button-shaped A input", () => {
    const screen = new (TitleScreen as any)(makeMockGraphics(), makeMockAudio());

    screen.registers.setScx(0);
    screen.update();
    screen.update();

    screen.handleInput({ type: "keydown", button: "a", is_press: true } as any, true);
    screen.update();

    expect(screen.popAction()).toBe("main_menu");
  });

  it("queues main menu from configured A key codes used by scheduled input", () => {
    const screen = new (TitleScreen as any)(makeMockGraphics(), makeMockAudio());

    screen.registers.setScx(0);
    screen.update();
    screen.update();

    screen.handleInput({ type: "keydown", key: "KeyJ", code: "KeyJ", is_press: true } as any, true);
    screen.update();

    expect(screen.popAction()).toBe("main_menu");
  });

  it("requires a fresh same-frame delete-save combo instead of sequential held buttons", () => {
    const screen = new (TitleScreen as any)(makeMockGraphics(), makeMockAudio());

    screen.registers.setScx(0);
    screen.update();
    screen.update();

    screen.handleInput({ key: "ArrowUp" } as KeyboardEvent, true);
    screen.update();
    expect(screen.popAction()).toBeNull();

    screen.handleInput({ key: "b" } as KeyboardEvent, true);
    screen.update();
    expect(screen.popAction()).toBeNull();

    screen.handleInput({ key: "Select" } as KeyboardEvent, true);
    screen.update();
    expect(screen.popAction()).toBeNull();
  });

  it("requires a fresh same-frame clock-reset trigger combo instead of sequential held buttons", () => {
    const screen = new (TitleScreen as any)(makeMockGraphics(), makeMockAudio());

    screen.registers.setScx(0);
    screen.update();
    screen.update();

    screen.handleInput({ key: "ArrowDown" } as KeyboardEvent, true);
    screen.update();
    expect(screen.clockResetTrigger).toBe(false);

    screen.handleInput({ key: "b" } as KeyboardEvent, true);
    screen.update();
    expect(screen.clockResetTrigger).toBe(false);

    screen.handleInput({ key: "Select" } as KeyboardEvent, true);
    screen.update();
    expect(screen.clockResetTrigger).toBe(false);
  });

  it("describes the current title-screen state for text rendering", () => {
    const screen = new (TitleScreen as any)(makeMockGraphics(), makeMockAudio());
    screen.state = 2;
    screen.titleTimer = 123;

    const snapshot = screen.getTextSnapshot();

    expect(snapshot.viewportTitle).toBe("Title");
    expect(snapshot.viewportLines).toEqual(expect.arrayContaining(["POKEMON CRYSTAL", "PRESS START"]));
    expect(snapshot.infoLines).toEqual(
      expect.arrayContaining([
        "STATE: main",
        "TIMER: 123",
        "A/START=Main menu",
        "Up+B+Select=Delete save",
        "DOWN+B+SELECT arms reset clock",
      ])
    );
    expect(snapshot.promptLines).toBeNull();
  });
});
