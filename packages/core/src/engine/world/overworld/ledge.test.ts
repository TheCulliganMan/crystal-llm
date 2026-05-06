import { collectCollisionSamples } from "./ledge";

describe("ledge collision sampling", () => {
  it("returns no samples when a metatile id exceeds the active tileset bounds", () => {
    const map = {
      width: 3,
      height: 3,
      getMetatileAt: () => 10,
    };
    const tileset = {
      tilesetName: "players_house",
      metatiles: [{ collision: [0, 0, 0, 0] }],
      renderMetatile: jest.fn(),
      renderPriorityMetatile: jest.fn(),
    };

    expect(
      collectCollisionSamples(map as never, tileset as never, 0, 0, 2),
    ).toEqual([]);
  });
});
