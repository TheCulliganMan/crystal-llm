import { CompositeChild, CompositeUI } from "./composite-ui";
import { DomCanvasUI } from "./dom-canvas-ui";
import { TextUI } from "./text-ui";

export type RendererInstance = DomCanvasUI | TextUI | CompositeUI;

const AVAILABLE_RENDERERS = new Set(["canvas", "text", "ascii"]);

export const resolveBackendName = (rendererName: string | null): string => {
  const normalized = rendererName ? rendererName.toLowerCase().trim() : null;
  const resolved = normalized === "cli" ? "text" : normalized ?? "canvas";
  if (!AVAILABLE_RENDERERS.has(resolved)) {
    throw new Error(
      `Renderer '${resolved}' not available; compiled backends: ${[...AVAILABLE_RENDERERS].sort()}`
    );
  }
  return resolved;
};

export class RendererFactory {
  static build(
    rendererName: string | null,
    options: {
      scale: number;
      dualRender: boolean;
      textLiveMode: boolean;
      textDumpJson: boolean;
      textMarkX: number | null;
      textMarkY: number | null;
      textMarkChar: string;
      textRefreshHz: number | null;
    }
  ): [RendererInstance, TextUI | null, boolean, string, boolean] {
    const resolved = resolveBackendName(rendererName);
    let textUiTarget: TextUI | null = null;
    const textRendererActive =
      resolved === "ascii" || resolved === "text" || options.dualRender;
    let textDumpJson = options.textDumpJson;

    let ui: RendererInstance;
    if (options.dualRender) {
      const primaryUi = new DomCanvasUI(undefined, undefined, options.scale);
      textDumpJson = true;
      const textUi = new TextUI(undefined, undefined, options.scale, null, options.textLiveMode, options.textRefreshHz);
      if (options.textMarkX !== null && options.textMarkY !== null) {
        textUi.setMarker(options.textMarkX, options.textMarkY, options.textMarkChar);
      }
      textUiTarget = textUi;
      ui = new CompositeUI(primaryUi as unknown as CompositeChild, textUi as unknown as CompositeChild);
    } else if (resolved === "ascii" || resolved === "text") {
      const textUi = new TextUI(undefined, undefined, options.scale, null, options.textLiveMode, options.textRefreshHz, true);
      if (options.textMarkX !== null && options.textMarkY !== null) {
        textUi.setMarker(options.textMarkX, options.textMarkY, options.textMarkChar);
      }
      textUiTarget = textUi;
      ui = textUi;
    } else {
      const primaryUi = new DomCanvasUI(undefined, undefined, options.scale);
      const textUi = new TextUI(undefined, undefined, options.scale, null, false, 0, true);
      if (options.textMarkX !== null && options.textMarkY !== null) {
        textUi.setMarker(options.textMarkX, options.textMarkY, options.textMarkChar);
      }
      textUiTarget = textUi;
      ui = new CompositeUI(primaryUi as unknown as CompositeChild, textUi as unknown as CompositeChild);
    }

    const promptRendererName = options.dualRender ? resolved : textRendererActive ? "text" : resolved;
    return [ui, textUiTarget, textRendererActive, promptRendererName, textDumpJson];
  }
}

export type RendererFactoryMeta = typeof RendererFactory;
