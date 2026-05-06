import { Surface } from "./surface";
import { TextboxRenderer } from "./textbox";

describe("TextboxRenderer", () => {
  it("passes rich text options through the renderText fallback path", () => {
    const renderText = jest.fn();
    const ui = {
      tile_size: 8,
      default_frame_id: 1,
      font: {
        renderText,
      },
      get_context_palette: () => [
        [255, 255, 255],
        [170, 170, 170],
        [85, 85, 85],
        [0, 0, 0],
      ] as [number, number, number][],
      set_context_palette: jest.fn(),
      draw_window: jest.fn(),
    };
    const renderer = new TextboxRenderer(ui);
    const surface = new Surface(160, 144);

    renderer.drawTextBox(surface, "Mixed case text", 0, 0, 10, 4);

    expect(renderText).toHaveBeenCalledWith(
      "Mixed case text",
      8,
      16,
      surface,
      expect.objectContaining({
        textWidth: 64,
        maxLines: 2,
        uppercase: false,
        color: [0, 0, 0],
      }),
    );
  });
});
