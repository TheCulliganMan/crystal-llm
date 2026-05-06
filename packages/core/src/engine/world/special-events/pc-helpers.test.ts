import { createInitialGameState } from "@pokecrystal/core/core/state";
import { has_hall_of_fame_record, pc_hub_options } from "./pc-helpers";

describe("PC Hall of Fame helpers", () => {
  it("treats saved Hall of Fame entries as a record when the migrated count is absent", () => {
    const gameState = createInitialGameState();
    gameState.sram.hall_of_fame = [{ win_count: 1, team: [{ species: "CYNDAQUIL" }] }];
    gameState.wram.wHallOfFameCount = 0;

    expect(has_hall_of_fame_record(gameState)).toBe(true);
    expect(pc_hub_options(gameState)).toContain("HALL OF FAME");
  });
});
