import fs from "fs";
import { Surface } from "../surface";
import { PlayerGender } from "../../core/enums";

describe("bagTileset caching", () => {
  afterEach(() => {
    jest.resetModules();
    jest.restoreAllMocks();
  });

  it("reuses decoded pack assets across repeated draws for the same pocket and gender", async () => {
    jest.resetModules();
    const readSpy = jest.spyOn(fs, "readFileSync");
    const { bagTileset } = await import("./bag-menu-layout");

    const font = {
      fontTiles: { 1: new Surface(8, 8) },
      paletteVariants: jest.fn(() => ({})),
    };

    bagTileset(font, 0, PlayerGender.MALE);
    const firstReadCount = readSpy.mock.calls.length;

    bagTileset(font, 0, PlayerGender.MALE);

    expect(readSpy.mock.calls.length).toBe(firstReadCount);
  });
});
