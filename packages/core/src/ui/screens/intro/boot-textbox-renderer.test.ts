import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { BootTextboxRenderer } from "./boot-textbox-renderer";

describe("BootTextboxRenderer", () => {
  const originalLoadSync = gameEngine.image.loadSync;

  afterEach(() => {
    gameEngine.image.loadSync = originalLoadSync;
    jest.restoreAllMocks();
  });

  it("matches the shared frame tile mapping for window borders", () => {
    const frameSurface = new gameEngine.Surface(24, 16);
    frameSurface.fill([255, 255, 255, 0]);
    frameSurface.fill([255, 0, 0, 255], new gameEngine.Rect(0, 0, 8, 8));
    frameSurface.fill([0, 255, 0, 255], new gameEngine.Rect(8, 0, 8, 8));
    frameSurface.fill([0, 0, 255, 255], new gameEngine.Rect(16, 0, 8, 8));
    frameSurface.fill([255, 255, 0, 255], new gameEngine.Rect(0, 8, 8, 8));
    frameSurface.fill([255, 0, 255, 255], new gameEngine.Rect(8, 8, 8, 8));
    frameSurface.fill([0, 255, 255, 255], new gameEngine.Rect(16, 8, 8, 8));

    gameEngine.image.loadSync = jest.fn(() => frameSurface);

    const renderer = new BootTextboxRenderer({
      render_text: jest.fn(),
      renderText: jest.fn(),
      get_char_tile: jest.fn(),
      getCharTile: jest.fn(),
    });

    const target = new gameEngine.Surface(32, 32);
    (renderer as unknown as {
      drawWindowSurface: (
        surface: InstanceType<typeof gameEngine.Surface>,
        xPx: number,
        yPx: number,
        widthTiles: number,
        heightTiles: number
      ) => void;
    }).drawWindowSurface(target, 0, 0, 3, 3);

    expect(target.get_at([0, 0]).slice(0, 3)).toEqual([255, 0, 0]);
    expect(target.get_at([8, 0]).slice(0, 3)).toEqual([0, 255, 0]);
    expect(target.get_at([16, 0]).slice(0, 3)).toEqual([0, 0, 255]);
    expect(target.get_at([0, 8]).slice(0, 3)).toEqual([255, 255, 0]);
    expect(target.get_at([16, 8]).slice(0, 3)).toEqual([255, 255, 0]);
    expect(target.get_at([0, 16]).slice(0, 3)).toEqual([255, 0, 255]);
    expect(target.get_at([8, 16]).slice(0, 3)).toEqual([0, 255, 0]);
    expect(target.get_at([16, 16]).slice(0, 3)).toEqual([0, 255, 255]);
  });
});
