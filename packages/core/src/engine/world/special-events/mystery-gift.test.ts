import { createInitialGameState } from "@pokecrystal/core/core/state";
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { get_mystery_gift_item } from "./mystery-gift";

describe("get_mystery_gift_item", () => {
  it("accepts TM_HM_* stored items and clears the pending gift", () => {
    const gameState = createInitialGameState();
    const dataLoader = new DataLoader();
    const itemSystem = new ItemSystem(gameState, dataLoader);
    gameState.sram.mystery_gift.stored_item = "TM_HM_01";
    gameState.sram.mystery_gift.backup_item = "TM_HM_01";

    const added = get_mystery_gift_item({
      game_state: gameState,
      runner: { item_system: itemSystem } as any,
    });

    expect(added).toBe(true);
    expect(gameState.sram.mystery_gift.stored_item).toBeNull();
    expect(gameState.sram.mystery_gift.backup_item).toBeNull();
    expect(gameState.sram.tm_hm[0]).toBe(1);
  });
});
