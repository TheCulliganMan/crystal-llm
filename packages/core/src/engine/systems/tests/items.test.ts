import { ItemSystem, MAX_ITEM_STACK, POCKET_CAPACITY } from "../items";
import { GameState, createInitialGameState } from "../../../core/state";
import { ItemEffect, ItemPocket } from "../../../core/enums/item";
import { DataLoader } from "../../../core/data-loader";
import { Item } from "../../../core/models";
import { items as contentItems } from "@pokecrystal/assets/content/items";

const contentItem = (scriptName: string): Item => {
  const item = contentItems.find((candidate) => candidate.script_name === scriptName);
  if (!item) {
    throw new Error(`Missing test content item ${scriptName}`);
  }
  return item;
};

const testItem = (overrides: Partial<Item>): Item => ({
  ...contentItem("POTION"),
  ...overrides,
});

describe("ItemSystem", () => {
  let gameState: GameState;
  let itemSystem: ItemSystem;
  let dataLoader: DataLoader;

  beforeEach(() => {
    gameState = createInitialGameState();
    dataLoader = {
      itemData: new Map<string, Item>(contentItems.map((item) => [item.script_name, item])),
    } as DataLoader;
    itemSystem = new ItemSystem(gameState, dataLoader);
  });

  describe("addItem", () => {
    it("rejects non-positive quantities", () => {
      expect(() => itemSystem.addItem("POTION", 0)).toThrow("quantity must be a positive integer");
      expect(() => itemSystem.addItem("POTION", -1)).toThrow("quantity must be a positive integer");
    });

    it("rejects display-name aliases instead of coercing them to script names", () => {
      expect(() => itemSystem.addItem("Great Ball", 1)).toThrow("Unknown item definition: Great Ball");
    });

    it("handles exact canonical script names", () => {
        expect(itemSystem.addItem("MASTER_BALL", 1)).toBe(true);
        expect(gameState.sram.balls["MASTER_BALL"]).toBe(1);
    });

    it("loads a missing canonical definition when the loader cache is only partially populated", () => {
      const silverWing = testItem({
        name: "SILVER WING",
        script_name: "SILVER_WING",
        pocket: ItemPocket.KEY_ITEM,
      });
      const partialLoader = {
        itemData: new Map([["POTION", contentItem("POTION")]]),
        get_item: jest.fn((name: string) => name === "SILVER_WING" ? silverWing : null),
      };
      const partialItemSystem = new ItemSystem(gameState, partialLoader);

      expect(partialItemSystem.addItem("SILVER_WING")).toBe(true);
      expect(partialLoader.get_item).toHaveBeenCalledWith("SILVER_WING");
      expect(gameState.sram.key_items.SILVER_WING).toBe(1);
    });

    it("adds an item to the correct pocket", () => {
      expect(itemSystem.addItem("POTION", 1)).toBe(true);
      expect(gameState.sram.items["POTION"]).toBe(1);
    });

    it("increments the quantity of an existing item", () => {
      itemSystem.addItem("POTION", 1);
      itemSystem.addItem("POTION", 2);
      expect(gameState.sram.items["POTION"]).toBe(3);
    });

    it("respects MAX_ITEM_STACK", () => {
      gameState.sram.items["POTION"] = MAX_ITEM_STACK - 1;
      expect(itemSystem.addItem("POTION", 1)).toBe(true);
      expect(gameState.sram.items["POTION"]).toBe(MAX_ITEM_STACK);
      expect(itemSystem.addItem("POTION", 1)).toBe(false);
    });

    it("respects pocket capacity for new items", () => {
        const capacity = POCKET_CAPACITY[ItemPocket.ITEM]!;
        const itemNames = contentItems
            .filter((item) => item.pocket === ItemPocket.ITEM && item.script_name !== "POTION")
            .slice(0, capacity)
            .map((item) => item.script_name);
        expect(itemNames).toHaveLength(capacity);
        for (const itemName of itemNames) {
            itemSystem.addItem(itemName, 1);
        }
        expect(Object.keys(gameState.sram.items).length).toBe(capacity);
        expect(itemSystem.addItem("POTION", 1)).toBe(false);
    });


    it("adds a key item", () => {
      expect(itemSystem.addItem("BICYCLE", 1)).toBe(true);
      expect(gameState.sram.key_items["BICYCLE"]).toBe(1);
    });

    it("does not stack key items", () => {
      itemSystem.addItem("BICYCLE", 1);
      expect(itemSystem.addItem("BICYCLE", 1)).toBe(false);
      expect(gameState.sram.key_items["BICYCLE"]).toBe(1);
    });

    it("adds a ball", () => {
      expect(itemSystem.addItem("POKE_BALL", 5)).toBe(true);
      expect(gameState.sram.balls["POKE_BALL"]).toBe(5);
    });

    it("adds a TM", () => {
      expect(itemSystem.addItem("TM_HM_01", 1)).toBe(true);
      expect(itemSystem.hasItem("TM_HM_01")).toBe(true);
      expect(gameState.sram.tm_hm[0]).toBe(1);
    });

    it("rejects collapsed asm-style item aliases without underscore", () => {
      expect(() => itemSystem.addItem("PSNCUREBERRY", 1)).toThrow("Unknown item definition: PSNCUREBERRY");
    });

    it("adds exact item script names that look like legacy aliases", () => {
      expect(itemSystem.addItem("BLACK_BELT", 1)).toBe(true);
      expect(gameState.sram.items["BLACK_BELT"]).toBe(1);
    });

    it("returns false if TM is already present", () => {
      itemSystem.addItem("TM_HM_01", 1);
      expect(itemSystem.addItem("TM_HM_01", 1)).toBe(false);
    });

    it("accepts item objects directly", () => {
      const potion = contentItem("POTION");

      expect(itemSystem.addItem(potion, 2)).toBe(true);
      expect(itemSystem.getQuantity(potion)).toBe(2);
      expect(gameState.sram.items["POTION"]).toBe(2);
    });

    it("tracks held vs non-held metadata while sharing item inventory behavior", () => {
      const inventoryLoader = {
        itemData: new Map<string, Item>([
          [
            "CUSTOM_HELD_ITEM",
            testItem({
              name: "CUSTOM HELD ITEM",
              pocket: ItemPocket.ITEM,
              price: 500,
              description: "An item with a held effect.",
              effect: "NONE",
              parameter: 0,
              script_name: "CUSTOM_HELD_ITEM",
              held_effect: "HELD_WATER_BOOST",
              property: "",
              field_menu: "",
              battle_menu: "",
            }),
          ],
          [
            "CUSTOM_NON_HELD_ITEM",
            testItem({
              name: "CUSTOM NON HELD ITEM",
              pocket: ItemPocket.ITEM,
              price: 500,
              description: "An item without a held effect.",
              effect: "NONE",
              parameter: 0,
              script_name: "CUSTOM_NON_HELD_ITEM",
              held_effect: "HELD_NONE",
              property: "",
              field_menu: "",
              battle_menu: "",
            }),
          ],
        ]),
      } as DataLoader;
      const inventorySystem = new ItemSystem(gameState, inventoryLoader);

      expect(inventorySystem.addItem("CUSTOM_HELD_ITEM", 4)).toBe(true);
      expect(inventorySystem.getItemDefinition("CUSTOM_HELD_ITEM").held_effect).toBe("HELD_WATER_BOOST");
      expect(inventorySystem.addItem("CUSTOM_NON_HELD_ITEM", 2)).toBe(true);
      expect(inventorySystem.getItemDefinition("CUSTOM_NON_HELD_ITEM").held_effect).toBe("HELD_NONE");
      expect(() => inventorySystem.getQuantity("custom held item")).toThrow("Unknown item definition: custom held item");
      expect(inventorySystem.getQuantity("CUSTOM_NON_HELD_ITEM")).toBe(2);
    });

    it("supports exact TM/HM item keys", () => {
      expect(itemSystem.addItem("TM_HM_01", 1)).toBe(true);
      expect(gameState.sram.tm_hm[0]).toBe(1);
      expect(itemSystem.addItem("TM_HM_01", 1)).toBe(false);

      expect(itemSystem.addItem("TM_HM_51", 1)).toBe(true);
      expect(gameState.sram.tm_hm[50]).toBe(1);
    });

    it("respects pocket capacity for BALL and KEY_ITEM pockets", () => {
      const ballCapacity = POCKET_CAPACITY[ItemPocket.BALL]!;
      const ballNames = contentItems
        .filter((item) => item.pocket === ItemPocket.BALL)
        .slice(0, ballCapacity)
        .map((item) => item.script_name);
      expect(ballNames).toHaveLength(ballCapacity);
      for (const itemName of ballNames) {
        expect(itemSystem.addItem(itemName, 1)).toBe(true);
      }
      expect(Object.keys(gameState.sram.balls).length).toBe(ballCapacity);
      (dataLoader.itemData as Map<string, Item>).set(
        "SURPLUS_BALL",
        testItem({
          name: "SURPLUS BALL",
          script_name: "SURPLUS_BALL",
          pocket: ItemPocket.BALL,
          effect: "POKE_BALL",
        }),
      );
      expect(itemSystem.addItem("SURPLUS_BALL", 1)).toBe(false);

      const keyCapacity = POCKET_CAPACITY[ItemPocket.KEY_ITEM]!;
      const keyNames = contentItems
        .filter((item) => item.pocket === ItemPocket.KEY_ITEM)
        .slice(0, keyCapacity)
        .map((item) => item.script_name);
      for (let index = keyNames.length; index < keyCapacity; index += 1) {
        const scriptName = `CUSTOM_KEY_${index}`;
        (dataLoader.itemData as Map<string, Item>).set(
          scriptName,
          testItem({
            name: scriptName.replace(/_/g, " "),
            script_name: scriptName,
            pocket: ItemPocket.KEY_ITEM,
            effect: "NONE",
            consumable: false,
          }),
        );
        keyNames.push(scriptName);
      }
      expect(keyNames).toHaveLength(keyCapacity);
      for (const itemName of keyNames) {
        expect(itemSystem.addItem(itemName, 1)).toBe(true);
      }
      expect(Object.keys(gameState.sram.key_items).length).toBe(keyCapacity);
      expect(() => itemSystem.addItem("UNKNOWN_CARD", 1)).toThrow("Unknown item definition: UNKNOWN_CARD");
    });
  });

  describe("removeItem", () => {
    it("removes an item completely", () => {
      itemSystem.addItem("POTION", 5);
      expect(itemSystem.removeItem("POTION", 5)).toBe(true);
      expect(gameState.sram.items["POTION"]).toBeUndefined();
    });

    it("decrements the quantity of an item", () => {
      itemSystem.addItem("POTION", 5);
      expect(itemSystem.removeItem("POTION", 2)).toBe(true);
      expect(gameState.sram.items["POTION"]).toBe(3);
    });

    it("removes the item when asked to remove more than owned", () => {
      itemSystem.addItem("POTION", 5);
      expect(itemSystem.removeItem("POTION", 6)).toBe(true);
      expect(gameState.sram.items["POTION"]).toBeUndefined();
    });

    it("removes a TM", () => {
        itemSystem.addItem("TM_HM_01", 1);
        expect(itemSystem.removeItem("TM_HM_01", 1)).toBe(true);
        expect(itemSystem.hasItem("TM_HM_01")).toBe(false);
        expect(gameState.sram.tm_hm[0]).toBe(0);
    });

    it("does not remove an HM", () => {
        itemSystem.addItem("TM_HM_51", 1);
        expect(itemSystem.removeItem("TM_HM_51", 1)).toBe(false);
        expect(itemSystem.hasItem("TM_HM_51")).toBe(true);
    });

    it("removes TMs but not HMs using exact item keys", () => {
      expect(itemSystem.removeItem("TM_HM_01", 1)).toBe(false);

      itemSystem.addItem("TM_HM_01", 1);
      expect(itemSystem.removeItem("TM_HM_01", 1)).toBe(true);
      expect(itemSystem.hasItem("TM_HM_01")).toBe(false);

      itemSystem.addItem("TM_HM_51", 1);
      expect(itemSystem.removeItem("TM_HM_51", 1)).toBe(false);
      expect(itemSystem.hasItem("TM_HM_51")).toBe(true);
    });

    it("rejects non-positive quantities", () => {
      expect(() => itemSystem.removeItem("POTION", 0)).toThrow("quantity must be a positive integer");
      expect(() => itemSystem.removeItem("POTION", -1)).toThrow("quantity must be a positive integer");
    });
  });

  describe("getQuantity", () => {
    it("returns the correct quantity for an item", () => {
      itemSystem.addItem("POTION", 15);
      expect(itemSystem.getQuantity("POTION")).toBe(15);
    });

    it("returns 0 for an unowned item", () => {
      expect(itemSystem.getQuantity("ELIXER")).toBe(0);
    });

    it("returns 1 for a known TM", () => {
        itemSystem.addItem("TM_HM_01", 1);
        expect(itemSystem.getQuantity("TM_HM_01")).toBe(1);
    });
  });

  describe("lookup helpers", () => {
    it("infers pockets for item categories", () => {
      expect(itemSystem.getItemPocket("POTION")).toBe(ItemPocket.ITEM);
      expect(itemSystem.getItemPocket("POKE_BALL")).toBe(ItemPocket.BALL);
      expect(itemSystem.getItemPocket("BICYCLE")).toBe(ItemPocket.KEY_ITEM);
      expect(itemSystem.getItemPocket("TM_HM_01")).toBe(ItemPocket.TM_HM);
      expect(itemSystem.getItemPocket("TM_HM_51")).toBe(ItemPocket.TM_HM);
    });

    it("rejects unknown item names instead of inferring pockets from suffixes", () => {
      expect(() => itemSystem.getItemPocket("TRAINERS_CARD")).toThrow("Unknown item definition: TRAINERS_CARD");
      expect(() => itemSystem.getItemPocket("RIDE_PASS")).toThrow("Unknown item definition: RIDE_PASS");
      expect(() => itemSystem.getItemPocket("MAGNET_GEAR")).toThrow("Unknown item definition: MAGNET_GEAR");
      expect(() => itemSystem.getItemPocket("SPECIALTICKET")).toThrow("Unknown item definition: SPECIALTICKET");
    });

    it("classifies every exact content key item as a key item", () => {
      const contentSystem = new ItemSystem(gameState);
      const keyItems = contentItems.filter((item) => item.pocket === ItemPocket.KEY_ITEM);

      expect(keyItems.map((item) => item.script_name).sort()).toEqual([
        "BASEMENT_KEY",
        "BICYCLE",
        "BLUE_CARD",
        "CARD_KEY",
        "CLEAR_BELL",
        "COIN_CASE",
        "EGG_TICKET",
        "GOOD_ROD",
        "GS_BALL",
        "ITEMFINDER",
        "LOST_ITEM",
        "MACHINE_PART",
        "MYSTERY_EGG",
        "OLD_ROD",
        "PASS",
        "SECRET_POTION",
        "SQUIRT_BOTTLE",
        "SUPER_ROD",
        "S_S_TICKET",
        "TOWN_MAP",
      ]);
      for (const keyItem of keyItems) {
        expect(contentSystem.getItemPocket(keyItem.script_name)).toBe(ItemPocket.KEY_ITEM);
        expect(contentSystem.addItem(keyItem.script_name)).toBe(true);
        expect(contentSystem.addItem(keyItem.script_name)).toBe(false);
        expect(contentSystem.getQuantity(keyItem.script_name)).toBe(1);
      }

      expect(() => contentSystem.getItemPocket("SQUIRTBOTTLE")).toThrow("Unknown item definition: SQUIRTBOTTLE");
      expect(() => contentSystem.getItemPocket("S.S. TICKET")).toThrow("Unknown item definition: S.S. TICKET");
    });

    it("formats display names for exact canonical, TM, and HM items", () => {
      expect(itemSystem.getDisplayName("POTION")).toBe("Potion");
      expect(itemSystem.getDisplayName("MASTER_BALL")).toBe("Master Ball");
      expect(itemSystem.getDisplayName("TM_HM_01")).toBe("TM01");
      expect(itemSystem.getDisplayName("TM_HM_51")).toBe("HM01");
      expect(() => itemSystem.getDisplayName("mystery_item")).toThrow("Unknown item definition: mystery_item");
    });

    it("uses item_data records when provided", () => {
      const recordLoader = {
        item_data: {
          POTION: {
            name: "POTION",
            pocket: ItemPocket.ITEM,
            price: 300,
            description: "Restores HP.",
            effect: ItemEffect.RESTORE_HP,
            parameter: 0,
            script_name: "POTION",
            held_effect: "HELD_NONE",
            property: "",
            field_menu: "",
            battle_menu: "",
          },
        },
      } as DataLoader;
      const recordSystem = new ItemSystem(gameState, recordLoader);

      expect(recordSystem.getItemDefinition("POTION").effect).toBe(ItemEffect.RESTORE_HP);
    });
  });

  describe("listItems", () => {
      it("lists items for a specific pocket", () => {
          itemSystem.addItem("POKE_BALL", 10);
          itemSystem.addItem("GREAT_BALL", 5);
          const balls = itemSystem.listItems(ItemPocket.BALL);
          expect(balls).toEqual({
              "POKE_BALL": 10,
              "GREAT_BALL": 5,
          });
      });

      it("lists all items when no pocket is specified", () => {
          itemSystem.addItem("POTION", 2);
          itemSystem.addItem("BICYCLE", 1);
          itemSystem.addItem("POKE_BALL", 10);
          itemSystem.addItem("TM_HM_01", 1);
          const allItems = itemSystem.listItems();
          expect(allItems).toEqual({
              "POTION": 2,
              "BICYCLE": 1,
              "POKE_BALL": 10,
              "TM_HM_01": 1,
          });
      });
  });

  describe("bundled item definitions", () => {
    it("uses exact bundled content items when no data loader is provided", () => {
      const contentSystem = new ItemSystem(gameState);
      const potion = contentSystem.getItemDefinition("POTION");
      expect(potion.script_name).toBe("POTION");
      expect(potion.effect).toBe(ItemEffect.RESTORE_HP);
    });

    it("uses loader item effects as authored without hydrating from bundled content data", () => {
      const potion = itemSystem.getItemDefinition("POTION");
      expect(potion.effect).toBe(ItemEffect.RESTORE_HP);
      expect(potion.parameter).toBe(20);
    });

    it("rejects empty itemData Map loaders instead of hydrating bundled content", () => {
      const emptyMapLoader = { itemData: new Map<string, Item>() } as DataLoader;
      const seededSystem = new ItemSystem(gameState, emptyMapLoader);

      expect(() => seededSystem.getItemDefinition("ELIXER")).toThrow("Unknown item definition: ELIXER");
      expect((emptyMapLoader.itemData as Map<string, Item>).size).toBe(0);
    });

    it("rejects empty item_data record loaders instead of hydrating bundled content", () => {
      const emptyRecordLoader = { item_data: {} } as DataLoader;
      const seededSystem = new ItemSystem(gameState, emptyRecordLoader);

      expect(() => seededSystem.getItemDefinition("ELIXER")).toThrow("Unknown item definition: ELIXER");
      expect((emptyRecordLoader.item_data as Record<string, Item>).ELIXER).toBeUndefined();
    });

    it("uses itemData Map entries directly", () => {
      const mapLoader = {
        itemData: new Map<string, Item>([
          [
            "CUSTOM_MAP_ITEM",
            testItem({
              name: "CUSTOM MAP ITEM",
              pocket: ItemPocket.ITEM,
              price: 420,
              description: "Map sourced item.",
              effect: ItemEffect.REVIVE,
              parameter: 0,
              script_name: "CUSTOM_MAP_ITEM",
              held_effect: "HELD_NONE",
              property: "",
              field_menu: "",
              battle_menu: "",
            }),
          ],
        ]),
      } as DataLoader;
      const mapSystem = new ItemSystem(gameState, mapLoader);

      const customItem = mapSystem.getItemDefinition("CUSTOM_MAP_ITEM");
      expect(customItem.effect).toBe(ItemEffect.REVIVE);
      expect(mapSystem.addItem("CUSTOM_MAP_ITEM", 1)).toBe(true);
      expect(mapSystem.getQuantity("CUSTOM_MAP_ITEM")).toBe(1);
    });
  });

  describe("tm/hm inventory listing", () => {
    it("lists TM/HM entries from the bitflag inventory", () => {
      itemSystem.addItem("TM_HM_01", 1);
      itemSystem.addItem("TM_HM_51", 1);

      expect(itemSystem.listItems(ItemPocket.TM_HM)).toEqual({
        TM_HM_01: 1,
        TM_HM_51: 1,
      });
    });
  });
});
