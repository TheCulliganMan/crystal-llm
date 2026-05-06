import { CompositeChild, CompositeUI } from "@pokecrystal/core/ui/composite-ui";
import { DomCanvasUI } from "@pokecrystal/core/ui/dom-canvas-ui";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import { assertAsmScale, assertAsmUiInvariants } from "@/app/asm-rendering-invariants";

export type RendererMode = "tile" | "both" | "text";

export interface UiOptions {
  scale: number;
  rendererMode: RendererMode;
}

export type UiBuildResult = {
  ui: BaseUI;
  textUi: TextUI | null;
};

export function buildUi(
  canvas: HTMLCanvasElement,
  options: Partial<UiOptions> = {}
): UiBuildResult {
  const scale = options.scale ?? 1;
  assertAsmScale(scale, "buildUi");
  const rendererMode = options.rendererMode ?? "tile";
  if (rendererMode === "text") {
    const textUi = new TextUI(undefined, undefined, scale, null, false, 0, true);
    assertAsmUiInvariants(textUi as unknown as BaseUI, "buildUi:text");
    return { ui: textUi, textUi };
  }
  const tileUi = new DomCanvasUI(undefined, undefined, scale, undefined, canvas);
  assertAsmUiInvariants(tileUi as unknown as BaseUI, "buildUi:tile");
  const textUi = new TextUI(undefined, undefined, scale, null, false, 0, true);
  assertAsmUiInvariants(textUi as unknown as BaseUI, "buildUi:text");
  const composite = new CompositeUI(
    tileUi as unknown as CompositeChild,
    textUi as unknown as CompositeChild,
  );
  return { ui: composite as unknown as BaseUI, textUi };
}
