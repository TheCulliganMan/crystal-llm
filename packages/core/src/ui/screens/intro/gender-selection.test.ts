import { PlayerGender } from "@pokecrystal/core/core/enums";
import { HeadlessCanvas } from "@pokecrystal/core/ui/headless-canvas";
import { GenderSelectionScreen } from "./gender-selection";

describe("GenderSelectionScreen", () => {
  const createFont = () => ({
    render_text: jest.fn(),
    renderText: jest.fn(),
    get_char_tile: jest.fn(),
    getCharTile: jest.fn(),
  });

  const renderAfterFade = (screen: GenderSelectionScreen, canvas: HeadlessCanvas) => {
    for (let i = 0; i < GenderSelectionScreen.FADE_IN_FRAMES; i += 1) {
      screen.update();
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Failed to create headless render context.");
    }
    screen.draw(ctx as unknown as CanvasRenderingContext2D);
    return ctx;
  };

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

  it("renders the dedicated gender screen background across the full target canvas", () => {
    const screen = new GenderSelectionScreen(createFont());
    const canvas = new HeadlessCanvas(320, 288);
    const ctx = renderAfterFade(screen, canvas);

    expect(Array.from(ctx.getImageData(0, 0, 1, 1).data)).toEqual([74, 247, 255, 255]);
    expect(Array.from(ctx.getImageData(319, 0, 1, 1).data)).toEqual([74, 247, 255, 255]);
  });
});
