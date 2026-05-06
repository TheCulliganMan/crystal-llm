import { TextUI } from "./text-ui";

describe("TextUI", () => {
  it("updates only at the configured refresh interval when live mode is enabled", () => {
    const ui = new TextUI(10, 8, 1, null, true, 2);
    ui.renderSnapshot(["A"], ["B"]);

    const flush = jest.spyOn(ui as unknown as { flush_window_stack: () => void }, "flush_window_stack");
    const now = jest.spyOn(performance, "now");
    now.mockReturnValueOnce(600).mockReturnValueOnce(800).mockReturnValueOnce(1200);

    ui.update();
    ui.update();
    ui.update();

    expect(flush).toHaveBeenCalledTimes(2);
  });

  it("flushes the window stack when closing and clears snapshot state", () => {
    const ui = new TextUI(10, 8);
    ui.renderSnapshot(["A"], ["B"]);
    ui.setActionLog(["a", "b"]);
    ui.setMarker(1, 2, "X");
    const flush = jest.spyOn(ui as unknown as { flush_window_stack: () => void }, "flush_window_stack");
    ui.close();

    expect(ui.getSnapshot()).toBeNull();
    expect(flush).toHaveBeenCalled();
  });

  it("reuses the previous snapshot object when visible text content is unchanged", () => {
    const ui = new TextUI(10, 8);

    ui.renderSnapshot(["OVERWORLD"], ["INFO"], "Viewport", "Info", ["MENU"], ["PROMPT"], ["DIALOGUE"]);
    const firstSnapshot = ui.getSnapshot();

    ui.renderSnapshot(["OVERWORLD"], ["INFO"], "Viewport", "Info", ["MENU"], ["PROMPT"], ["DIALOGUE"]);

    expect(ui.getSnapshot()).toBe(firstSnapshot);
  });

  it("publishes a new snapshot object when visible text content changes", () => {
    const ui = new TextUI(10, 8);

    ui.renderSnapshot(["OVERWORLD"], ["INFO"]);
    const firstSnapshot = ui.getSnapshot();

    ui.renderSnapshot(["BATTLE"], ["INFO"]);

    expect(ui.getSnapshot()).not.toBe(firstSnapshot);
  });

  it("does not clear the hidden backing surface when text snapshots redraw", () => {
    const ui = new TextUI(10, 8);
    const fill = jest.spyOn(ui.screen, "fill");

    ui.clearScreen([0, 0, 0]);

    expect(fill).not.toHaveBeenCalled();
  });
});
