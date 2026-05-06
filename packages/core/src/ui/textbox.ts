import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { Rect, Surface as UISurface } from "@pokecrystal/core/ui/surface";
import type { Palette, RenderTextOptions } from "@pokecrystal/core/ui/font-renderer";

type RGB = [number, number, number];

type FontSurface = InstanceType<typeof gameEngine.Surface> | UISurface;

type FontRenderOptions = {
  text_width?: number;
  max_lines?: number;
  uppercase?: boolean;
  color?: RGB;
  palette?: RGB[];
  [key: string]: unknown;
};

export type FontRenderer = {
  font_tiles?: Record<number, FontSurface>;
  fontTiles?: Record<number, FontSurface>;
  paletteVariants?: (palettes: readonly Palette[]) => Record<number, Record<number, FontSurface>>;
  set_frame_tiles?: (frameId: number) => Promise<void>;
  setFrameTiles?: (frameId: number) => Promise<void>;
  render_text?: (
    text: string,
    x: number,
    y: number,
    surface: FontSurface,
    options?: FontRenderOptions,
  ) => void;
  renderText?: (
    text: string,
    x: number,
    y: number,
    surface: FontSurface,
    options?: RenderTextOptions | boolean,
  ) => void;
  get_char_tile?: (char: string) => FontSurface | null | undefined;
  getCharTile?: (char: string) => FontSurface | null | undefined;
};

type TextboxUI = {
  tile_size?: number;
  tileSize?: number;
  default_frame_id?: number;
  defaultFrameId?: number;
  font: FontRenderer;
  get_context_palette?: (name: string) => RGB[];
  getContextPalette?: (name: string) => RGB[];
  set_context_palette?: (name: string, palette: RGB[]) => RGB[];
  setContextPalette?: (name: string, palette: RGB[]) => RGB[];
  draw_window?: (
    surface: FontSurface,
    x: number,
    y: number,
    width: number,
    height: number,
    options?: {
      frame_id?: number;
      fill?: RGB;
      z_index?: number;
      record?: boolean;
    },
  ) => void;
  drawWindow?: (
    surface: FontSurface,
    x: number,
    y: number,
    width: number,
    height: number,
    options?: {
      frameId?: number;
      fill?: RGB;
      zIndex?: number;
      record?: boolean;
    },
  ) => void;
  _record_window_region?: (
    surface: FontSurface,
    x: number,
    y: number,
    width: number,
    height: number,
    zIndex: number,
  ) => void;
  _recordWindowRegion?: (
    surface: FontSurface,
    x: number,
    y: number,
    width: number,
    height: number,
    zIndex: number,
  ) => void;
};

export class TextboxRenderer {
  private readonly ui: TextboxUI;
  private readonly borderTileCache = new Map<string, FontSurface>();

  constructor(ui: TextboxUI) {
    this.ui = ui;
  }

  drawTextBox(
    surface: FontSurface,
    text: string,
    x: number,
    y: number,
    width: number,
    height: number,
    opts?: {
      frameId?: number;
      fill?: RGB;
      textColor?: RGB;
      zIndex?: number;
    },
  ): void {
    const textboxPalette = this.getContextPalette("textbox");
    if (!textboxPalette.length) {
      throw new Error("Textbox palette is empty.");
    }
    if (width < 2 || height < 2) {
      throw new Error("Textbox border requires at least 2x2 tiles");
    }
    const fillColor = opts?.fill ?? textboxPalette[0];
    const tintColor = opts?.textColor ?? textboxPalette[textboxPalette.length - 1];
    const xPx = this.tilesToPixels(x, "x");
    const yPx = this.tilesToPixels(y, "y");
    const resolvedFrame = opts?.frameId ?? this.getDefaultFrameId();
    this.drawWindow(surface, xPx, yPx, width, height, resolvedFrame, fillColor, opts?.zIndex ?? 0);
    this.applyTextboxPalette(textboxPalette);
    const tileSize = this.getTileSize();
    const textX = xPx + tileSize;
    const textY = yPx + 2 * tileSize;
    const textWidth = Math.max(0, (width - 2) * tileSize);
    const maxLines = Math.max(0, height - 2);
    this.renderText(text, textX, textY, surface, {
      color: tintColor,
      palette: textboxPalette,
      text_width: textWidth,
      max_lines: maxLines,
      uppercase: false,
    });
    this.recordWindowRegion(surface, xPx, yPx, width, height, opts?.zIndex ?? 0);
  }

  private tilesToPixels(tiles: number, axis: string): number {
    if (tiles < 0) {
      throw new Error(`${axis}-tile coordinate cannot be negative`);
    }
    return tiles * this.getTileSize();
  }

  private drawWindow(
    surface: FontSurface,
    xPx: number,
    yPx: number,
    width: number,
    height: number,
    frameId: number,
    fill: RGB,
    zIndex: number,
  ): void {
    if (this.ui.draw_window) {
      this.ui.draw_window(surface, xPx, yPx, width, height, {
        frame_id: frameId,
        fill,
        z_index: zIndex,
        record: false,
      });
      return;
    }
    if (this.ui.drawWindow) {
      this.ui.drawWindow(surface, xPx, yPx, width, height, {
        frameId,
        fill,
        zIndex,
        record: false,
      });
      return;
    }
    throw new Error("TextboxRenderer requires UI.draw_window to render.");
  }

  private drawTextboxBorderChars(
    surface: FontSurface,
    x: number,
    y: number,
    width: number,
    height: number,
    borderColor: RGB,
    fillColor: RGB,
  ): void {
    if (width < 2 || height < 2) {
      throw new Error("Textbox border requires at least 2x2 tiles");
    }
    const tileSize = this.getTileSize();
    const innerWidth = Math.max(0, width - 2) * tileSize;
    const innerHeight = Math.max(0, height - 2) * tileSize;
    if (innerWidth && innerHeight) {
      const innerRect = new Rect(
        x + tileSize,
        y + tileSize,
        innerWidth,
        innerHeight,
      );
      surface.fill([fillColor[0], fillColor[1], fillColor[2], 255], innerRect);
    }

    const left = x;
    const right = x + (width - 1) * tileSize;
    const top = y;
    const bottom = y + (height - 1) * tileSize;

    this.blitBorderTile(surface, "┌", borderColor, [left, top]);
    this.blitBorderTile(surface, "┐", borderColor, [right, top]);
    this.blitBorderTile(surface, "└", borderColor, [left, bottom]);
    this.blitBorderTile(surface, "┘", borderColor, [right, bottom]);

    for (let col = 1; col < width - 1; col += 1) {
      const xPos = x + col * tileSize;
      this.blitBorderTile(surface, "─", borderColor, [xPos, top]);
      this.blitBorderTile(surface, "─", borderColor, [xPos, bottom]);
    }

    for (let row = 1; row < height - 1; row += 1) {
      const yPos = y + row * tileSize;
      this.blitBorderTile(surface, "│", borderColor, [left, yPos]);
      this.blitBorderTile(surface, "│", borderColor, [right, yPos]);
    }
  }

  private applyTextboxPalette(palette: RGB[]): void {
    if (this.ui.set_context_palette) {
      this.ui.set_context_palette("textbox", palette);
      return;
    }
    if (this.ui.setContextPalette) {
      this.ui.setContextPalette("textbox", palette);
      return;
    }
    throw new Error("TextboxRenderer requires UI.set_context_palette to render.");
  }

  private getContextPalette(name: string): RGB[] {
    if (this.ui.get_context_palette) {
      return this.ui.get_context_palette(name);
    }
    if (this.ui.getContextPalette) {
      return this.ui.getContextPalette(name);
    }
    throw new Error("TextboxRenderer requires UI.get_context_palette to render.");
  }

  private getDefaultFrameId(): number {
    if (this.ui.default_frame_id !== undefined) {
      return this.ui.default_frame_id;
    }
    if (this.ui.defaultFrameId !== undefined) {
      return this.ui.defaultFrameId;
    }
    return 0;
  }

  private getTileSize(): number {
    return this.ui.tile_size ?? this.ui.tileSize ?? 8;
  }

  private recordWindowRegion(
    surface: FontSurface,
    xPx: number,
    yPx: number,
    width: number,
    height: number,
    zIndex: number,
  ): void {
    if (this.ui._record_window_region) {
      this.ui._record_window_region(surface, xPx, yPx, width, height, zIndex);
    } else if (this.ui._recordWindowRegion) {
      this.ui._recordWindowRegion(surface, xPx, yPx, width, height, zIndex);
    }
  }

  private renderText(
    text: string,
    x: number,
    y: number,
    surface: FontSurface,
    options: {
      text_width: number;
      max_lines: number;
      uppercase: boolean;
      color: RGB;
      palette: RGB[];
    },
  ): void {
    if (this.ui.font.render_text) {
      this.ui.font.render_text(text, x, y, surface, {
        text_width: options.text_width,
        max_lines: options.max_lines,
        uppercase: options.uppercase,
        color: options.color,
        palette: options.palette,
      });
      return;
    }
    if (this.ui.font.renderText) {
      this.ui.font.renderText(text, x, y, surface, {
        textWidth: options.text_width,
        maxLines: options.max_lines,
        uppercase: options.uppercase,
        color: options.color,
        palette: options.palette,
      });
      return;
    }
    throw new Error("TextboxRenderer requires UI.font.render_text or UI.font.renderText to render.");
  }

  private blitBorderTile(
    surface: FontSurface,
    char: string,
    color: RGB,
    position: [number, number],
  ): void {
    const key = `${char}:${color.join(",")}`;
    let tinted = this.borderTileCache.get(key);
    if (!tinted) {
      const tile = this.getCharTile(char);
      if (!tile) {
        throw new Error(`Textbox border character ${JSON.stringify(char)} is missing from the font`);
      }
      tinted = this.colorizeSurface(tile, color);
      this.borderTileCache.set(key, tinted);
    }
    const blitter = (surface as { blit: (tile: FontSurface, position: [number, number]) => void }).blit;
    if (tinted) {
      blitter(tinted, position);
    }
  }

  private getCharTile(char: string): FontSurface | null | undefined {
    if (this.ui.font.get_char_tile) {
      return this.ui.font.get_char_tile(char);
    }
    if (this.ui.font.getCharTile) {
      return this.ui.font.getCharTile(char);
    }
    return null;
  }

  private colorizeSurface(
    surface: FontSurface,
    color: RGB,
  ): FontSurface {
    const tinted = surface.copy();
    const [width, height] = tinted.get_size();
    for (let row = 0; row < height; row += 1) {
      for (let col = 0; col < width; col += 1) {
        const pixel = tinted.get_at([col, row]);
        if (pixel[3] === 0) {
          continue;
        }
        tinted.set_at([col, row], [color[0], color[1], color[2], pixel[3]]);
      }
    }
    return tinted;
  }
}
