import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { TextboxRenderer, type FontRenderer as TextboxFontRenderer } from "@pokecrystal/core/ui/textbox";

type RGB = [number, number, number];
type Surface = InstanceType<typeof gameEngine.Surface>;

const DEFAULT_TEXTBOX_PALETTE: RGB[] = [
  [255, 255, 255],
  [170, 170, 170],
  [85, 85, 85],
  [0, 0, 0],
];
const DEFAULT_FRAME_ID = 1;

type FrameTiles = Record<`${0 | 1 | 2},${0 | 1}`, Surface>;

const ensureCanvasImageSource = (surface: Surface): CanvasImageSource => {
  return surface.canvas as CanvasImageSource;
};

export class BootTextboxRenderer {
  private readonly textboxRenderer: TextboxRenderer;
  private textboxPalette = [...DEFAULT_TEXTBOX_PALETTE];
  private frameTiles: FrameTiles | null = null;

  constructor(
    private readonly font: TextboxFontRenderer,
    private readonly tileSize: number = 8
  ) {
    this.textboxRenderer = new TextboxRenderer({
      tile_size: tileSize,
      default_frame_id: 1,
      font: this.font,
      get_context_palette: () => [...this.textboxPalette],
      set_context_palette: (_name, palette) => {
        this.textboxPalette = [...palette];
        return [...this.textboxPalette];
      },
      draw_window: (surface, x, y, width, height, options) => {
        this.drawWindowSurface(surface as Surface, x, y, width, height, options?.fill);
      },
    });
  }

  drawTextBox(
    ctx: CanvasRenderingContext2D,
    text: string,
    xTiles: number,
    yTiles: number,
    widthTiles: number,
    heightTiles: number
  ): void {
    const surface = this.createOverlaySurface(ctx);
    this.textboxRenderer.drawTextBox(
      surface,
      text,
      xTiles,
      yTiles,
      widthTiles,
      heightTiles
    );
    ctx.drawImage(ensureCanvasImageSource(surface), 0, 0);
  }

  drawWindow(
    ctx: CanvasRenderingContext2D,
    xTiles: number,
    yTiles: number,
    widthTiles: number,
    heightTiles: number
  ): void {
    const surface = this.createOverlaySurface(ctx);
    this.drawWindowSurface(
      surface,
      xTiles * this.tileSize,
      yTiles * this.tileSize,
      widthTiles,
      heightTiles,
      this.textboxPalette[0]
    );
    ctx.drawImage(ensureCanvasImageSource(surface), 0, 0);
  }

  drawText(
    ctx: CanvasRenderingContext2D,
    text: string,
    xPx: number,
    yPx: number,
    options: {
      textWidth?: number;
      maxLines?: number;
      color?: RGB;
      uppercase?: boolean;
    } = {}
  ): void {
    const surface = this.createOverlaySurface(ctx);
    if (typeof this.font.render_text === "function") {
      this.font.render_text(text, xPx, yPx, surface, {
        text_width: options.textWidth,
        max_lines: options.maxLines,
        color: options.color ?? this.textboxPalette[3],
        palette: this.textboxPalette,
        uppercase: options.uppercase ?? false,
      });
    } else if (typeof this.font.renderText === "function") {
      this.font.renderText(text, xPx, yPx, surface, {
        textWidth: options.textWidth,
        maxLines: options.maxLines,
        color: options.color ?? this.textboxPalette[3],
        palette: this.textboxPalette,
        uppercase: options.uppercase ?? false,
      } as never);
    } else {
      throw new Error("BootTextboxRenderer requires a bitmap font renderer.");
    }
    ctx.drawImage(ensureCanvasImageSource(surface), 0, 0);
  }

  drawPromptArrow(
    ctx: CanvasRenderingContext2D,
    arrowSurface: Surface,
    xPx: number,
    yPx: number
  ): void {
    ctx.drawImage(ensureCanvasImageSource(arrowSurface), xPx, yPx);
  }

  private createOverlaySurface(ctx: CanvasRenderingContext2D): Surface {
    const surface = new gameEngine.Surface(ctx.canvas.width, ctx.canvas.height);
    surface.fill([255, 255, 255, 0]);
    return surface;
  }

  private drawWindowSurface(
    surface: Surface,
    xPx: number,
    yPx: number,
    widthTiles: number,
    heightTiles: number,
    fill: RGB = this.textboxPalette[0]
  ): void {
    const frameTiles = this.loadFrameTiles();
    if (frameTiles) {
      const left = xPx;
      const right = xPx + (widthTiles - 1) * this.tileSize;
      const top = yPx;
      const bottom = yPx + (heightTiles - 1) * this.tileSize;

      surface.blit(frameTiles["0,0"], [left, top]);
      surface.blit(frameTiles["2,0"], [right, top]);
      surface.blit(frameTiles["1,1"], [left, bottom]);
      surface.blit(frameTiles["2,1"], [right, bottom]);

      for (let column = 1; column < widthTiles - 1; column += 1) {
        const x = xPx + column * this.tileSize;
        surface.blit(frameTiles["1,0"], [x, top]);
        surface.blit(frameTiles["1,0"], [x, bottom]);
      }

      for (let row = 1; row < heightTiles - 1; row += 1) {
        const y = yPx + row * this.tileSize;
        surface.blit(frameTiles["0,1"], [left, y]);
        surface.blit(frameTiles["0,1"], [right, y]);
      }

      if (widthTiles > 2 && heightTiles > 2) {
        surface.fill(
          [fill[0], fill[1], fill[2], 255],
          new gameEngine.Rect(
            xPx + this.tileSize,
            yPx + this.tileSize,
            (widthTiles - 2) * this.tileSize,
            (heightTiles - 2) * this.tileSize
          )
        );
      }
      return;
    }

    const widthPx = widthTiles * this.tileSize;
    const heightPx = heightTiles * this.tileSize;
    surface.fill(
      [fill[0], fill[1], fill[2], 255],
      new gameEngine.Rect(xPx, yPx, widthPx, heightPx)
    );

    const left = xPx;
    const right = xPx + (widthTiles - 1) * this.tileSize;
    const top = yPx;
    const bottom = yPx + (heightTiles - 1) * this.tileSize;

    this.blitChar(surface, "┌", left, top);
    this.blitChar(surface, "┐", right, top);
    this.blitChar(surface, "└", left, bottom);
    this.blitChar(surface, "┘", right, bottom);

    for (let column = 1; column < widthTiles - 1; column += 1) {
      const x = xPx + column * this.tileSize;
      this.blitChar(surface, "─", x, top);
      this.blitChar(surface, "─", x, bottom);
    }

    for (let row = 1; row < heightTiles - 1; row += 1) {
      const y = yPx + row * this.tileSize;
      this.blitChar(surface, "│", left, y);
      this.blitChar(surface, "│", right, y);
    }
  }

  private blitChar(surface: Surface, char: string, x: number, y: number): void {
    const tile =
      this.font.get_char_tile?.(char) ??
      this.font.getCharTile?.(char) ??
      null;
    if (!tile) {
      throw new Error(`Boot textbox glyph ${JSON.stringify(char)} is missing.`);
    }
    surface.blit(tile as Surface, [x, y]);
  }

  private loadFrameTiles(): FrameTiles | null {
    if (this.frameTiles) {
      return this.frameTiles;
    }
    const loader = gameEngine.image.loadSync;
    if (typeof loader !== "function") {
      return null;
    }
    const frameSurface = loader(getAssetPath("gfx", "frames", `${DEFAULT_FRAME_ID}.png`));
    if (!frameSurface) {
      return null;
    }
    const expectedWidth = this.tileSize * 3;
    const expectedHeight = this.tileSize * 2;
    const [width, height] = frameSurface.get_size();
    if (width !== expectedWidth || height !== expectedHeight) {
      throw new Error(
        `Boot textbox frame tileset must be ${expectedWidth}x${expectedHeight}, got ${width}x${height}.`
      );
    }
    this.frameTiles = {
      "0,0": frameSurface.subsurface(new gameEngine.Rect(0, 0, this.tileSize, this.tileSize)).copy(),
      "1,0": frameSurface.subsurface(new gameEngine.Rect(this.tileSize, 0, this.tileSize, this.tileSize)).copy(),
      "2,0": frameSurface.subsurface(new gameEngine.Rect(this.tileSize * 2, 0, this.tileSize, this.tileSize)).copy(),
      "0,1": frameSurface.subsurface(new gameEngine.Rect(0, this.tileSize, this.tileSize, this.tileSize)).copy(),
      "1,1": frameSurface.subsurface(new gameEngine.Rect(this.tileSize, this.tileSize, this.tileSize, this.tileSize)).copy(),
      "2,1": frameSurface.subsurface(new gameEngine.Rect(this.tileSize * 2, this.tileSize, this.tileSize, this.tileSize)).copy(),
    };
    return this.frameTiles;
  }
}
