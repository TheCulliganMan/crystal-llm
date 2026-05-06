import { CompositeUI } from "./composite-ui";

describe("CompositeUI", () => {
  it("broadcasts renderSnapshot to child UIs", () => {
    const childA = { renderSnapshot: jest.fn() };
    const childB = { renderSnapshot: jest.fn() };
    const composite = new CompositeUI(childA, childB) as CompositeUI & {
      renderSnapshot: (
        viewportLines: string[] | null,
        infoLines: string[] | null,
        viewportTitle: string,
        infoTitle: string,
        menuLines?: string[] | null,
        promptLines?: string[] | null,
        dialogueLines?: string[] | null
      ) => void;
    };

    composite.renderSnapshot(["VIEW"], ["INFO"], "Viewport", "Info", null, null, null);

    expect(childA.renderSnapshot).toHaveBeenCalledWith(
      ["VIEW"],
      ["INFO"],
      "Viewport",
      "Info",
      null,
      null,
      null
    );
    expect(childB.renderSnapshot).toHaveBeenCalledWith(
      ["VIEW"],
      ["INFO"],
      "Viewport",
      "Info",
      null,
      null,
      null
    );
  });

  it("mirrors font assignments onto child UIs", () => {
    const childA = { font: null as any };
    const childB = { font: null as any };
    const composite = new CompositeUI(childA as any, childB as any) as CompositeUI & { font?: any };
    const font = { renderText: jest.fn() };

    composite.font = font;

    expect(childA.font).toBe(font);
    expect(childB.font).toBe(font);
  });

  it("returns snapshot data from a later child UI", () => {
    const childA = {};
    const snapshot = { viewportTitle: "Prompt", dialogueLines: ["What day is it?"] };
    const childB = { getSnapshot: jest.fn(() => snapshot) };
    const composite = new CompositeUI(childA as any, childB as any) as CompositeUI & {
      getSnapshot: () => typeof snapshot;
    };

    expect(composite.getSnapshot()).toBe(snapshot);
    expect(childB.getSnapshot).toHaveBeenCalledTimes(1);
  });

  it("calls non-broadcast drawing methods on only the primary child", () => {
    const primary = { drawWindow: jest.fn() };
    const secondary = { drawWindow: jest.fn() };
    const composite = new CompositeUI(primary as any, secondary as any) as CompositeUI & {
      drawWindow: (surface: unknown, x: number, y: number, width: number, height: number) => void;
    };

    composite.drawWindow(null, 1, 2, 3, 4);

    expect(primary.drawWindow).toHaveBeenCalledWith(null, 1, 2, 3, 4);
    expect(secondary.drawWindow).not.toHaveBeenCalled();
  });
});
