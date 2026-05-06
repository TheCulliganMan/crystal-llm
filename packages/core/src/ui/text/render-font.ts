import type { RenderTextOptions, SurfaceLike } from "@pokecrystal/core/ui/font-renderer";

export type FontRendererLike = {
  render_text?: (text: string, x: number, y: number, surface: any, options?: any) => void;
  renderText?: (text: string, x: number, y: number, surface: any, options?: any) => void;
};

export const renderFontText = (
  font: FontRendererLike | null | undefined,
  text: string,
  x: number,
  y: number,
  surface: SurfaceLike | any,
  options?: RenderTextOptions | boolean,
): void => {
  if (font?.render_text) {
    font.render_text(text, x, y, surface, typeof options === "boolean" ? { uppercase: options } : options);
    return;
  }
  if (font?.renderText) {
    font.renderText(text, x, y, surface, options);
    return;
  }
  throw new Error("A font renderer with render_text or renderText is required.");
};
