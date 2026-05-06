const originalEnv = process.env;

describe("loadMoveMetadata", () => {
  beforeEach(() => {
    jest.resetModules();
    process.env = { ...originalEnv };
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  it("accepts null stat/amount entries from move data", () => {
    jest.doMock("../../core/asset-reader", () => ({
      readJsonAssetSync: jest.fn(() => ({
        TACKLE: {
          name: "TACKLE",
          type: "NORMAL",
          power: 40,
          accuracy: 100,
          pp: 35,
          stat: null,
          amount: null,
        },
        GROWL: {
          name: "GROWL",
          type: "NORMAL",
          power: 0,
          accuracy: 100,
          pp: 40,
          stat: "ATTACK",
          amount: null,
        },
      })),
    }));

    const { loadMoveMetadata } = require("./battle-experience");
    const data = loadMoveMetadata();
    expect(data.get("TACKLE")).toMatchObject({ name: "TACKLE" });
    expect(data.get("GROWL")).toMatchObject({ name: "GROWL", stat: "ATTACK" });
  });
});
