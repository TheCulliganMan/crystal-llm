import { createInitialGameState } from "@pokecrystal/core/core/state";
import { clearTemporaryEventFlags } from "./temporary-event-flags";

describe("clearTemporaryEventFlags", () => {
  it("clears temporary flags across WRAM and SRAM without touching others", () => {
    const gameState = createInitialGameState();
    gameState.wram.event_flags.EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1 = true;
    gameState.wram.event_flags.EVENT_TEMPORARY_UNTIL_MAP_RELOAD_2 = false;
    gameState.wram.event_flags.EVENT_OTHER_FLAG = true;
    gameState.sram.event_flags = { EVENT_TEMPORARY_UNTIL_MAP_RELOAD_2: true, EVENT_OTHER_FLAG: true };

    const cleared = clearTemporaryEventFlags(gameState);

    expect(gameState.wram.event_flags.EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1).toBe(false);
    expect(gameState.wram.event_flags.EVENT_TEMPORARY_UNTIL_MAP_RELOAD_2).toBe(false);
    expect(gameState.wram.event_flags.EVENT_OTHER_FLAG).toBe(true);
    expect(gameState.sram.event_flags.EVENT_TEMPORARY_UNTIL_MAP_RELOAD_2).toBe(false);
    expect(gameState.sram.event_flags.EVENT_OTHER_FLAG).toBe(true);
    expect(cleared.sort()).toEqual([
      "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_1",
      "EVENT_TEMPORARY_UNTIL_MAP_RELOAD_2",
    ]);
  });
});
