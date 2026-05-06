import { DomCanvasUI } from "./dom-canvas-ui";

describe("DomCanvasUI", () => {
  it("updates canvas image and flushes window stack without a redundant clear", () => {
    const drawImage = jest.fn();
    const mockCanvas = {
      width: 0,
      height: 0,
      style: {} as CSSStyleDeclaration,
      getContext: jest.fn().mockReturnValue({
        drawImage,
        imageSmoothingEnabled: false,
      }),
    } as unknown as HTMLCanvasElement;
    const ui = new DomCanvasUI(10, 8, 2, null, mockCanvas);
    const flush = jest.spyOn(ui as unknown as { flush_window_stack: () => void }, "flush_window_stack");

    ui.update();

    expect(mockCanvas.getContext).toHaveBeenCalledWith("2d", { willReadFrequently: true });
    expect(drawImage).toHaveBeenCalled();
    expect(flush).toHaveBeenCalled();
  });
});
