import {
  GameButton,
  isCancelEvent,
  isConfirmEvent,
  isSelectEvent,
  isStartEvent,
  mapKeyToButton,
} from "./buttons";
import { keycodes } from "../core/keycodes";

const keydown = (key: number | string) => ({ type: "keydown", key });

describe("button helpers", () => {
  it("maps keycodes to the expected Game Boy buttons", () => {
    expect(mapKeyToButton(keycodes.Enter)).toBe(GameButton.Start);
    expect(mapKeyToButton(keycodes.Backspace)).toBe(GameButton.Select);
    expect(mapKeyToButton(keycodes.ShiftLeft)).toBeNull();
    expect(mapKeyToButton(keycodes.ShiftRight)).toBeNull();
    expect(mapKeyToButton(keycodes.KeyZ)).toBe(GameButton.A);
    expect(mapKeyToButton(keycodes.KeyX)).toBe(GameButton.B);
  });

  it("treats Start and Select as distinct from confirm/cancel", () => {
    expect(isConfirmEvent(keydown(keycodes.Enter))).toBe(false);
    expect(isStartEvent(keydown(keycodes.Enter))).toBe(true);
    expect(isCancelEvent(keydown(keycodes.Escape))).toBe(false);
    expect(isSelectEvent(keydown(keycodes.Escape))).toBe(true);
  });
});
