import {
  GameButton,
  isConfirmEvent,
  mapKeyToButton,
  mapKeyToDirection,
} from "./controls";

describe("controls input normalization", () => {
  it("accepts direct player button names as game buttons", () => {
    expect(mapKeyToButton("a")).toBe(GameButton.A);
    expect(mapKeyToButton("B")).toBe(GameButton.B);
    expect(isConfirmEvent({ type: "keydown", button: "a", is_press: true })).toBe(true);
  });

  it("accepts direct player direction tokens", () => {
    expect(mapKeyToDirection("up")).toBe("up");
    expect(mapKeyToDirection("DOWN")).toBe("down");
  });
});
