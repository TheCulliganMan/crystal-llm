import { Shop, MartItem } from "../shop";
import { GameState, createInitialGameState } from "../../../core/state";
import { ItemSystem } from "../items";
import { DataLoader } from "../../../core/data-loader";
import { ItemPocket } from "../../../core/enums/item";
import { MAX_MONEY } from "../../../core/constants";
import { ItemSchema, type Item } from "../../../core/models/item";

describe("Shop", () => {
  let gameState: GameState;
  let itemSystem: ItemSystem;
  let dataLoader: DataLoader;
  let shop: Shop;
  let mockMartData: Map<string, string[]>;
  let mockItemData: Map<string, Item>;
  const makeItem = (value: { name: string; price: number; pocket: ItemPocket; script_name?: string; effect?: string }): Item =>
    ItemSchema.parse({
      name: value.name,
      script_name: value.script_name ?? value.name,
      description: "",
      effect: value.effect ?? (value.pocket === ItemPocket.BALL ? "POKE_BALL" : "NONE"),
      status_heals: [],
      revive_hp_percent: null,
      party_revive_hp_percent: null,
      pp_restore_scope: null,
      pp_restore_points: null,
      pp_up_stages: null,
      vitamin_stat: null,
      vitamin_stat_exp: null,
      vitamin_max_stat_exp: null,
      rare_candy_level_gain: null,
      battle_stat_boost_stat: null,
      battle_stat_boost_stages: null,
      battle_escape_mode: null,
      battle_focus_energy: null,
      battle_stat_drop_guard: null,
      battle_stat_drop_guard_turns: null,
      confusion_heal: null,
      repel_steps: null,
      escape_rope_mode: null,
      price: value.price,
      held_effect: "HELD_NONE",
      parameter: 0,
      property: "",
      pocket: value.pocket,
      field_menu: "",
      field_usable: false,
      battle_menu: "",
      battle_usable: false,
      battle_capture_ball: null,
      consumable: value.pocket !== ItemPocket.KEY_ITEM,
      tmhm_index: null,
      tmhm_move: null,
    });

  beforeEach(() => {
    gameState = createInitialGameState();
    dataLoader = new DataLoader();
    itemSystem = new ItemSystem(gameState, dataLoader);
    shop = new Shop(gameState, itemSystem, dataLoader);

    mockMartData = new Map([["CHERRYGROVE_CITY", ["POKE_BALL", "POTION"]]]);
    dataLoader.martData = mockMartData;

    mockItemData = new Map();
    mockItemData.set("POKE_BALL", makeItem({
      name: "POKE BALL",
      price: 200,
      pocket: ItemPocket.BALL,
    }));
    mockItemData.set("POTION", makeItem({
      name: "POTION",
      price: 300,
      pocket: ItemPocket.ITEM,
    }));
    dataLoader.itemData = mockItemData;
  });

  test("should load inventory for a mart", () => {
    const inventory = shop.loadInventory("CHERRYGROVE_CITY");
    expect(inventory).toHaveLength(2);
    expect(inventory[0].identifier).toBe("POKE_BALL");
    expect(inventory[1].price).toBe(300);
  });

  test("should resolve ASM mart constants to exported mart labels", () => {
    dataLoader.martData = new Map([["MARTCHERRYGROVEDEX", ["POKE_BALL", "POTION"]]]);

    const inventory = shop.loadInventory("MART_CHERRYGROVE_DEX");

    expect(inventory.map((item) => item.identifier)).toEqual(["POKE_BALL", "POTION"]);
  });

  test("should build a buy menu with CANCEL option", () => {
    const menu = shop.buildBuyMenu("CHERRYGROVE_CITY");
    expect(menu).toHaveLength(3);
    expect(menu[2].identifier).toBe("CANCEL");
  });

  describe("maxBuyQuantity", () => {
    test("should return 0 if the item pocket is full of unique items", () => {
        gameState.sram.money = 1000;
      // Define the dummy items in the mock item data
      for (let i = 0; i < 20; i++) {
        const dummyId = `DUMMY_ITEM_${i}`;
        mockItemData.set(dummyId, makeItem({
          name: dummyId,
          price: 1,
          pocket: ItemPocket.ITEM,
        }));
        itemSystem.addItem(dummyId, 1);
      }

      const item: MartItem = {
        identifier: "POTION",
        displayName: "Potion",
        price: 10,
      };

      const maxQuantity = shop.maxBuyQuantity(item);
      expect(maxQuantity).toBe(0);
    });
  });

  describe("buyItem", () => {
    beforeEach(() => {
      gameState.sram.money = 1000;
    });

    test("should allow buying an item with enough money and space", () => {
      const item: MartItem = {
        identifier: "POTION",
        displayName: "Potion",
        price: 300,
      };
      const result = shop.buyItem(item, 2);
      expect(result.success).toBe(true);
      expect(gameState.sram.money).toBe(400);
      expect(itemSystem.getQuantity("POTION")).toBe(2);
    });

    test("should not allow buying an item with insufficient funds", () => {
      const item: MartItem = {
        identifier: "POTION",
        displayName: "Potion",
        price: 300,
      };
      const result = shop.buyItem(item, 4);
      expect(result.success).toBe(false);
      expect(result.message).toBe("You don't have enough money.");
      expect(gameState.sram.money).toBe(1000);
      expect(itemSystem.getQuantity("POTION")).toBe(0);
    });

    test("should not allow buying an item when the pack is full", () => {
      // Define the dummy items in the mock item data
      for (let i = 0; i < 20; i++) {
        const dummyId = `DUMMY_ITEM_${i}`;
        mockItemData.set(dummyId, makeItem({
          name: dummyId,
          price: 1,
          pocket: ItemPocket.ITEM,
        }));
        itemSystem.addItem(dummyId, 1);
      }

      const item: MartItem = {
        identifier: "POTION",
        displayName: "Potion",
        price: 10,
      };

      const result = shop.buyItem(item, 1);
      expect(result.success).toBe(false);
      expect(result.message).toBe("Your Pack is full.");
    });

    test("should return failure when buying zero or negative quantity", () => {
      const item: MartItem = {
        identifier: "POTION",
        displayName: "Potion",
        price: 10,
      };
      const resultZero = shop.buyItem(item, 0);
      expect(resultZero.success).toBe(false);
      expect(resultZero.message).toBe("Quantity must be positive.");

      const resultNegative = shop.buyItem(item, -1);
      expect(resultNegative.success).toBe(false);
      expect(resultNegative.message).toBe("Quantity must be positive.");
    });
  });

  describe("sellItem", () => {
    beforeEach(() => {
      gameState.sram.money = 500;
      itemSystem.addItem("POTION", 5);
    });

    test("should allow selling an item", () => {
      const item: MartItem = {
        identifier: "POTION",
        displayName: "Potion",
        price: 300,
      };
      const result = shop.sellItem(item, 3);
      expect(result.success).toBe(true);
      // sell price is half, so 150 * 3 = 450. 500 + 450 = 950
      expect(gameState.sram.money).toBe(950);
      expect(itemSystem.getQuantity("POTION")).toBe(2);
    });

    test("should not exceed MAX_MONEY when selling", () => {
      gameState.sram.money = MAX_MONEY - 100;
      mockItemData.set("RARE_CANDY", makeItem({
        name: "RARE CANDY",
        price: 1000,
        pocket: ItemPocket.ITEM,
      }));
      itemSystem.addItem("RARE_CANDY", 1);
      const item: MartItem = {
        identifier: "RARE_CANDY",
        displayName: "Rare Candy",
        price: 1000,
      };
      const result = shop.sellItem(item, 1);
      expect(result.success).toBe(true);
      expect(gameState.sram.money).toBe(MAX_MONEY);
    });

    test("should not allow selling more items than owned", () => {
      const item: MartItem = {
        identifier: "POTION",
        displayName: "Potion",
        price: 300,
      };
      const result = shop.sellItem(item, 10);
      expect(result.success).toBe(false);
      expect(result.message).toBe("Looks like you don't have that many.");
      expect(gameState.sram.money).toBe(500);
      expect(itemSystem.getQuantity("POTION")).toBe(5);
    });

    test("should return failure when selling zero or negative quantity", () => {
      const item: MartItem = {
        identifier: "POTION",
        displayName: "Potion",
        price: 300,
      };
      const resultZero = shop.sellItem(item, 0);
      expect(resultZero.success).toBe(false);
      expect(resultZero.message).toBe("Quantity must be positive.");

      const resultNegative = shop.sellItem(item, -1);
      expect(resultNegative.success).toBe(false);
      expect(resultNegative.message).toBe("Quantity must be positive.");
    });
  });
});
