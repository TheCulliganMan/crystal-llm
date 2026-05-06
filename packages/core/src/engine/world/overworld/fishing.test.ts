import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { createInitialGameState } from "@pokecrystal/core/core/state";
import { roll_fishing_encounter } from "@pokecrystal/core/engine/world/overworld/fishing";

describe("Fishing encounter parity", () => {
  it("throws instead of defaulting invalid fishing time of day to day encounters", () => {
    const loader = new DataLoader();
    loader.map_attributes.set("TEST_MAP", {
      fishing_group: "FISHGROUP_LAKE",
    } as never);

    const gameState = createInitialGameState();
    gameState.wram.time_of_day = "twilight";

    const rng = {
      nextByte: jest.fn().mockReturnValueOnce(0).mockReturnValueOnce(231),
    } as never;

    expect(() =>
      roll_fishing_encounter(gameState, loader, "TEST_MAP", "GOOD_ROD", rng)
    ).toThrow("Unknown fishing time of day 'twilight'.");
  });
});
