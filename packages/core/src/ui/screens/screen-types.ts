import type { Surface } from "@pokecrystal/core/ui/surface";
import type { RenderTextOptions } from "@pokecrystal/core/ui/font-renderer";

export type RGB = [number, number, number];
export type RGBA = [number, number, number, number];

export interface UIFont {
  renderText(
    text: string,
    x: number,
    y: number,
    surface: Surface | import("@pokecrystal/core/ui/font-renderer").SurfaceLike,
    options?: RenderTextOptions | boolean
  ): void;
  paletteVariants?(
    palettes: RGB[][]
  ): Record<number, Record<number, Surface>>;
}

export interface UISnapshot {
  viewportLines?: string[] | null;
  infoLines?: string[] | null;
  menuLines?: string[] | null;
  promptLines?: string[] | null;
  dialogueLines?: string[] | null;
}

export interface ScreenUI {
  screen: Surface | null;
  font: UIFont;
  getPokemonFrontSurface?(speciesId: string, frame?: number): Surface | null;
  clearScreen?(color: RGB): void;
  drawBox?(
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number
  ): void;
  drawTextBox?(
    surface: Surface,
    text: string,
    x: number,
    y: number,
    width: number,
    height: number,
    zIndex?: number
  ): void;
  renderSnapshot?(
    viewportLines: string[] | null,
    infoLines: string[] | null,
    viewportTitle: string,
    infoTitle: string,
    menuLines?: string[] | null,
    promptLines?: string[] | null,
    dialogueLines?: string[] | null
  ): void;
  getSnapshot?(): UISnapshot | null;
  update?(): void;
  close?(): void;
  _recordWindowRegion?(
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    zIndex: number
  ): void;
}

export interface CompositeUI {
  children: ScreenUI[];
}

export const isTextUI = (ui: ScreenUI | CompositeUI | null): ui is ScreenUI => {
  return !!ui && typeof (ui as ScreenUI).renderSnapshot === "function";
};
