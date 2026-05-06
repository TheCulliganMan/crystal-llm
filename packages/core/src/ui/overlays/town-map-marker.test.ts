import { Surface } from "@pokecrystal/core/ui/surface";
import { drawTownMapCursorMarker } from "@pokecrystal/core/ui/overlays/town-map-marker";

describe("drawTownMapCursorMarker", () => {
  it("draws the ASM pokegear cursor as a 16x16 sprite centered on the landmark", () => {
    const surface = new Surface(32, 32);
    surface.fill([255, 255, 255, 255]);

    drawTownMapCursorMarker(surface, [16, 16]);

    expect(surface.getAt(8, 8)).toEqual([0, 0, 0, 255]);
    expect(surface.getAt(14, 8)).toEqual([0, 0, 0, 255]);
    expect(surface.getAt(16, 8)).toEqual([255, 255, 255, 255]);
    expect(surface.getAt(16, 16)).toEqual([255, 255, 255, 255]);
    expect(surface.getAt(8, 17)).toEqual([0, 0, 0, 255]);
    expect(surface.getAt(23, 23)).toEqual([0, 0, 0, 255]);
  });
});
