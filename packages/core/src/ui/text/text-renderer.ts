// ASM mapping: pokecrystal_disassembly/home/text.asm (PrintTextboxTextAt rendering flow).
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { BitmapFont, RenderTextOptions, RGB, Palette, SurfaceLike } from "./bitmap-font";
import { DEFAULT_TEXT_COLOUR } from "./colors";

type Surface = InstanceType<typeof gameEngine.Surface>;

export class TextRenderer {
  public readonly font: BitmapFont;
  public readonly using_bitmap = true;

  constructor() {
    this.font = new BitmapFont();
  }

  get font_tiles(): Record<number, Surface> {
    return this.font.font_tiles;
  }

  get fontTiles(): Record<number, Surface> {
    return this.font.fontTiles;
  }

  async load(): Promise<void> {
    await this.font.load();
  }

  paletteVariants(palettes: ReadonlyArray<Palette>): Record<number, Record<number, Surface>> {
    return this.font.paletteVariants(palettes);
  }

  getCharTile(char: string): Surface | null {
    return this.font.getCharTile(char);
  }

  renderText(
    text: string,
    x: number,
    y: number,
    surface: SurfaceLike,
    options?: RenderTextOptions | boolean
  ): void {
    this.font.renderText(text, x, y, surface, options);
  }

  render_text(
    text: string,
    x: number,
    y: number,
    surface: SurfaceLike,
    options?: RenderTextOptions
  ): void {
    this.font.render_text(text, x, y, surface, options);
  }

  drawText(
    surface: SurfaceLike,
    text: string,
    x: number,
    y: number,
    color: RGB = DEFAULT_TEXT_COLOUR
  ): void {
    this.font.render_text(text, x, y, surface, { color });
  }

  drawCenteredText(
    surface: SurfaceLike,
    text: string,
    y: number,
    color: RGB = DEFAULT_TEXT_COLOUR
  ): void {
    const width = text.length * this.font.charWidth;
    const surfaceWidth =
      "get_width" in surface && typeof surface.get_width === "function"
        ? surface.get_width()
        : "width" in surface && typeof surface.width === "number"
        ? surface.width
        : 0;
    const x = Math.floor((surfaceWidth - width) / 2);
    this.drawText(surface, text, x, y, color);
  }
}
