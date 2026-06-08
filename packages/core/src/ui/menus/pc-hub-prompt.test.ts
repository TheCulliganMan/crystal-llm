import { Surface } from "../surface";
import { PCHubMenu } from "./pc-hub-prompt";

describe("PCHubMenu", () => {
  it("draws the ASM top menu window and embedded prompt text", () => {
    const screen = new Surface(160, 144);
    const renderText = jest.fn();
    const drawWindow = jest.fn();
    const recordWindow = jest.fn();
    const ui = {
      screen,
      screenWidth: 160,
      screenHeight: 144,
      font: { renderText },
      drawWindow,
      _record_window_region: recordWindow,
    };

    const menu = new PCHubMenu(ui, ["BILL'S PC", "TURN OFF"], null, {
      promptText: "Access whose PC?",
    });

    menu.draw();

    expect(drawWindow).toHaveBeenCalledWith(
      screen,
      0,
      0,
      16,
      13,
      expect.objectContaining({ zIndex: expect.any(Number) }),
    );
    expect(recordWindow).toHaveBeenCalledWith(screen, 0, 0, 16, 13, expect.any(Number));
    expect(renderText).toHaveBeenNthCalledWith(
      1,
      "Access whose PC?",
      8,
      8,
      screen,
      expect.objectContaining({ uppercase: false, maxLines: 2 }),
    );
    expect(renderText).toHaveBeenNthCalledWith(
      2,
      "▶BILL'S PC",
      8,
      32,
      screen,
      expect.objectContaining({ uppercase: true }),
    );
  });

  it("binds renderText to the font instance", () => {
    const screen = new Surface(160, 144);
    const font = {
      renderText(
        this: unknown,
        _text: string,
        _x: number,
        _y: number,
        _surface: Surface,
        _options?: { textWidth?: number; maxLines?: number; uppercase?: boolean }
      ) {
        if (this !== font) {
          throw new Error("unbound renderText");
        }
      },
    };
    const ui = {
      screen,
      font,
      drawWindow: jest.fn(),
    };

    const menu = new PCHubMenu(ui, ["OPTION"]);

    expect(() => menu.draw()).not.toThrow();
  });

  it("emits a text-renderer snapshot for the PC hub menu", () => {
    const screen = new Surface(160, 144);
    const renderSnapshot = jest.fn();
    const ui = {
      screen,
      font: { renderText: jest.fn() },
      drawWindow: jest.fn(),
      renderSnapshot,
    };

    const menu = new PCHubMenu(ui, ["BILL'S PC", "TURN OFF"], null, {
      promptText: "Access whose PC?",
    });
    menu.draw();

    expect(renderSnapshot).toHaveBeenCalledWith(
      ["Access whose PC?"],
      ["D-Pad=Move A=Select B=Back"],
      "PC",
      "Legend",
      ["▶ BILL'S PC", "  TURN OFF"],
      null,
      null,
    );
  });

  it("handles MCP string key direction events", () => {
    const menu = new PCHubMenu(
      {
        screen: new Surface(160, 144),
        font: { renderText: jest.fn() },
        drawWindow: jest.fn(),
      },
      ["WITHDRAW", "DEPOSIT"],
    );

    menu.handleInput({ type: "keydown", key: "ArrowDown", code: "ArrowDown", is_press: true });

    expect(menu.index).toBe(1);
  });
});
