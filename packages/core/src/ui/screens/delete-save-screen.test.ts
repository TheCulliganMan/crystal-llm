import { Surface } from "@pokecrystal/core/ui/surface";
import { DeleteSaveScreen } from "./delete-save-screen";

describe("DeleteSaveScreen", () => {
  it("skips prompt box rendering for pure text UIs", () => {
    const ui = {
      screen: new Surface(160, 144),
      font: { renderText: jest.fn() },
      drawBox: jest.fn(),
      drawTextBox: jest.fn(),
      renderSnapshot: jest.fn(),
    };
    const screen = new DeleteSaveScreen(ui);

    screen.draw();

    expect(ui.drawBox).not.toHaveBeenCalled();
    expect(ui.drawTextBox).not.toHaveBeenCalled();
    expect(ui.renderSnapshot).toHaveBeenCalled();
  });
});
