import { buttonKeyCodes, mapKeycodeToButton } from "./bindings";
import { defaultKeyBindings, GameButton } from "./config";
import { keycodes } from "../core/keycodes";

describe("input bindings", () => {
  it("maps default key bindings to numeric keycodes", () => {
    for (const button of Object.values(GameButton)) {
      const numeric = buttonKeyCodes(button);
      const expected = (defaultKeyBindings[button] ?? [])
        .map((code) => keycodes[code])
        .filter((value): value is number => typeof value === "number");
      expect(numeric).toEqual(expected);
    }
  });

  it("resolves keycodes to the expected Game Boy buttons", () => {
    expect(mapKeycodeToButton(keycodes.Enter)).toBe(GameButton.Start);
    expect(mapKeycodeToButton(keycodes.Backspace)).toBe(GameButton.Select);
    expect(mapKeycodeToButton(keycodes.Escape)).toBe(GameButton.Select);
    expect(mapKeycodeToButton(keycodes.ShiftLeft)).toBeNull();
    expect(mapKeycodeToButton(keycodes.ShiftRight)).toBeNull();
    expect(mapKeycodeToButton(keycodes.KeyX)).toBe(GameButton.B);
  });
});
