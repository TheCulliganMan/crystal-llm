import { PlayerGender } from "@pokecrystal/core/core/enums";
import { GenderSelectionScreen } from "./gender-selection";

describe("GenderSelectionScreen", () => {
  const createFont = () => ({
    render_text: jest.fn(),
    renderText: jest.fn(),
    get_char_tile: jest.fn(),
    getCharTile: jest.fn(),
  });

  it("builds a text snapshot with the active selection highlighted", () => {
    const screen = new GenderSelectionScreen(createFont());

    const snapshot = screen.getTextSnapshot();

    expect(snapshot.viewportTitle).toBe("Gender");
    expect(snapshot.viewportLines).toEqual(expect.arrayContaining(["GENDER SELECT", "ARE YOU A BOY? OR ARE YOU A GIRL?"]));
    expect(snapshot.infoLines).toEqual(expect.arrayContaining(["STATE: gender", "SELECTION: boy", "Up/Down=Choose A=Confirm"]));
    expect(snapshot.menuLines).toEqual(["▶ BOY", "  GIRL"]);
    expect(snapshot.promptLines).toBeNull();
  });

  it("does not play a move sound when changing the selection", () => {
    const audioEngine = { playSound: jest.fn() } as any;
    const screen = new GenderSelectionScreen(createFont(), audioEngine);

    screen.handleInput({ type: "keydown", key: "ArrowDown" });

    expect(audioEngine.playSound).not.toHaveBeenCalled();
  });

  it("plays the confirm click sound and locks the selected gender", () => {
    const audioEngine = { playSound: jest.fn() } as any;
    const screen = new GenderSelectionScreen(createFont(), audioEngine);

    screen.handleInput({ type: "keydown", key: "ArrowDown" });
    screen.handleInput({ type: "keydown", code: "KeyZ", key: "z" });

    expect(audioEngine.playSound).toHaveBeenCalledWith("menu_option");
    expect(screen.isConfirmed()).toBe(true);
    expect(screen.getSelectedGender()).toBe(PlayerGender.FEMALE);
  });

  it("accepts direct player direction and button events", () => {
    const screen = new GenderSelectionScreen(createFont());

    screen.handleInput({ type: "keydown", direction: "down", is_press: true });
    screen.handleInput({ type: "keydown", button: "a", is_press: true });

    expect(screen.isConfirmed()).toBe(true);
    expect(screen.getSelectedGender()).toBe(PlayerGender.FEMALE);
  });
});
