const mockReadJsonAssetSync = jest.fn();
const mockGetSpawnPoint = jest.fn();

jest.mock("@pokecrystal/core/core/asset-reader", () => ({
  readJsonAssetSync: (...args: unknown[]) => mockReadJsonAssetSync(...args),
}));

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDataDir: () => "/mock/assets/data",
}));

jest.mock("@pokecrystal/core/core/path-utils", () => ({
  joinPath: (...parts: string[]) => parts.join("/"),
}));

jest.mock("@pokecrystal/core/engine/world/maps", () => ({
  Spawn: { HOME: "HOME" },
  getSpawnPoint: (...args: unknown[]) => mockGetSpawnPoint(...args),
  mapConstantToName: (mapConstant: string) => mapConstant,
}));

import { exportPlayability } from "./export-playability";

describe("exportPlayability", () => {
  beforeEach(() => {
    mockReadJsonAssetSync.mockReset();
    mockGetSpawnPoint.mockReset();
  });

  it("exports start position from runtime tile coordinates", () => {
    mockReadJsonAssetSync.mockImplementation((filePath: string) => {
      if (filePath.endsWith("story_events.json")) {
        return {};
      }
      if (filePath.endsWith("initialize_events.json")) {
        return {};
      }
      throw new Error(`unexpected asset read: ${filePath}`);
    });
    mockGetSpawnPoint.mockReturnValue({
      mapName: "PlayersHouse2F",
      tileX: 3,
      tileY: 3,
      metatileX: 1,
      metatileY: 1,
      subtileX: 1,
      subtileY: 1,
    });

    expect(exportPlayability().start_tiles).toEqual([
      { map: "PlayersHouse2F", tile: { x: 3, y: 3 } },
    ]);
  });
});
