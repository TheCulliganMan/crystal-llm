import { CompositeUI } from "./composite-ui";
import { DomCanvasUI } from "./dom-canvas-ui";
import { RendererFactory, resolveBackendName } from "./renderer-factory";
import { TextUI } from "./text-ui";

describe("resolveBackendName", () => {
  it("maps cli to text", () => {
    expect(resolveBackendName("cli")).toBe("text");
  });
});

describe("RendererFactory.build", () => {
  const baseOptions = {
    scale: 1,
    dualRender: false,
    textLiveMode: false,
    textDumpJson: false,
    textMarkX: null,
    textMarkY: null,
    textMarkChar: "@",
    textRefreshHz: 0,
  } as const;

  it("keeps a hidden text snapshot sink in canvas mode", () => {
    const [ui, textUi, textRendererActive, promptRendererName, textDumpJson] =
      RendererFactory.build("canvas", baseOptions);

    expect(ui).toBeInstanceOf(CompositeUI);
    expect(textUi).toBeInstanceOf(TextUI);
    expect((ui as CompositeUI).getPrimary()).toBeInstanceOf(DomCanvasUI);
    expect(textRendererActive).toBe(false);
    expect(promptRendererName).toBe("canvas");
    expect(textDumpJson).toBe(false);
  });

  it("still builds a composite UI in dual-render mode", () => {
    const [ui, textUi, textRendererActive, promptRendererName, textDumpJson] =
      RendererFactory.build("canvas", {
        ...baseOptions,
        dualRender: true,
      });

    expect(ui).toBeInstanceOf(CompositeUI);
    expect(textUi).toBeInstanceOf(TextUI);
    expect(textRendererActive).toBe(true);
    expect(promptRendererName).toBe("canvas");
    expect(textDumpJson).toBe(true);
  });
});
