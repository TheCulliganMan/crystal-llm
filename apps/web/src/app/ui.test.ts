/** @jest-environment jsdom */
import { buildUi } from "./ui";
import { CompositeUI } from "@pokecrystal/core/ui/composite-ui";
import { DomCanvasUI } from "@pokecrystal/core/ui/dom-canvas-ui";
import { TextUI } from "@pokecrystal/core/ui/text-ui";

describe("buildUi helper", () => {
  it("builds a tile renderer with a hidden snapshot UI when tile mode is requested", () => {
    const canvas = document.createElement("canvas");
    const { ui, textUi } = buildUi(canvas, { rendererMode: "tile", scale: 1 });
    expect(ui).toBeInstanceOf(CompositeUI);
    expect(textUi).toBeInstanceOf(TextUI);
    expect((ui as CompositeUI).getPrimary()).toBeInstanceOf(DomCanvasUI);
  });

  it("builds a composite UI when the both renderer is requested", () => {
    const canvas = document.createElement("canvas");
    const { ui, textUi } = buildUi(canvas, { rendererMode: "both", scale: 1 });
    expect(ui).toBeInstanceOf(CompositeUI);
    expect(textUi).toBeInstanceOf(TextUI);
  });

  it("builds a text UI when the text renderer is requested", () => {
    const canvas = document.createElement("canvas");
    const { ui, textUi } = buildUi(canvas, { rendererMode: "text", scale: 1 });
    expect(ui).toBeInstanceOf(TextUI);
    expect(textUi).toBe(ui);
  });

  it("builds passive text UIs because GameCanvas paints snapshots separately", () => {
    const canvas = document.createElement("canvas");
    const { ui } = buildUi(canvas, { rendererMode: "text", scale: 1 });
    const flush = jest.spyOn(ui as unknown as { flush_window_stack: () => void }, "flush_window_stack");

    (ui as TextUI).renderSnapshot(["OVERWORLD"], ["INFO"]);
    ui.update();

    expect(flush).not.toHaveBeenCalled();
  });

  it("rejects non-integer scale values", () => {
    const canvas = document.createElement("canvas");
    expect(() => buildUi(canvas, { rendererMode: "tile", scale: 1.25 })).toThrow(
      "Scale must be a positive integer",
    );
  });
});
