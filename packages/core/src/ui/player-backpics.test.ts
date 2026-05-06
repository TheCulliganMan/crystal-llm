import fs from "fs";
import path from "path";
import { gbcWordToRgb } from "@pokecrystal/core/core/gbc-colors";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { Surface } from "./surface";

type RGB = [number, number, number];

const mockDecode2bppTiles = jest.fn();

jest.mock("./2bpp", () => ({
  decode2bppTiles: (...args: unknown[]) => mockDecode2bppTiles(...args),
}));

const readTrainerPalette = (stem: string): RGB[] => {
  const palettePath = getAssetPath("gfx", "trainers", `${stem}.gbcpal`);
  const data = fs.readFileSync(path.resolve(palettePath));
  if (data.length < 8) {
    throw new Error(`Trainer palette ${palettePath} must be at least 8 bytes, got ${data.length}.`);
  }
  const colours: RGB[] = [];
  for (let offset = 0; offset < 8; offset += 2) {
    colours.push(gbcWordToRgb(data.readUInt16LE(offset)));
  }
  return colours;
};

describe("load_player_backpic_surface", () => {
  it("uses the trainer palette colors for the player backpic", () => {
    const palette = readTrainerPalette("cal");
    const expected = [palette[1], palette[2]];
    const tiles = Array.from({ length: 36 }, () => new Surface(8, 8));
    mockDecode2bppTiles.mockReturnValue(tiles);

    jest.isolateModules(() => {
      const { load_player_backpic_surface } = require("./player-backpics");
      load_player_backpic_surface("chris_back");
    });

    const paletteArg = mockDecode2bppTiles.mock.calls[0]?.[1] as RGB[] | undefined;
    expect(paletteArg).toBeDefined();
    expect(paletteArg?.[1]).toEqual(expected[0]);
    expect(paletteArg?.[2]).toEqual(expected[1]);
  });
});
