import { Surface } from "../surface";
import { assemble_place_graphic_surface } from "./place-graphic";

describe("assemble_place_graphic_surface", () => {
  it("lays out tiles row-major", () => {
    const tiles: Surface[] = [];
    for (let index = 0; index < 6; index += 1) {
      const tile = new Surface(1, 1);
      tile.fill([index, 0, 0, 255]);
      tiles.push(tile);
    }

    const blitSpy = jest.spyOn(Surface.prototype, "blit");

    assemble_place_graphic_surface(tiles, 3, 2);

    const targets = blitSpy.mock.calls.map((call) => call[1]);
    expect(targets).toEqual([[0, 0], [1, 0], [2, 0], [0, 1], [1, 1], [2, 1]]);

    blitSpy.mockRestore();
  });
});
