import { ItemSystem, MAX_ITEM_STACK, POCKET_CAPACITY } from "../items";
import { GameState, createInitialGameState } from "../../../core/state";
import { ItemEffect, ItemPocket } from "../../../core/enums/item";
import { DataLoader } from "../../../core/data-loader";
import { Item } from "../../../core/models";
import { items as contentItems } from "@pokecrystal/assets/content/items";

describe("ItemSystem", () => {
  let gameState: GameState;
  let itemSystem: ItemSystem;
  let dataLoader: DataLoader;

  beforeEach(() => {
    gameState = createInitialGameState();
    // Mock DataLoader with item definitions
    dataLoader = {
      itemData: new Map<string, Item>([
        ["POTION", { name: "POTION", pocket: ItemPocket.ITEM, price: 300, description: "Restores HP.", effect: ItemEffect.NONE, parameter: 0, script_name: "POTION", held_effect: "HELD_NONE", property: "", field_menu: "", battle_menu: "" }],
        ["BICYCLE", { name: "BICYCLE", pocket: ItemPocket.KEY_ITEM, price: 0, description: "A folding bicycle.", effect: ItemEffect.NONE, parameter: 0, script_name: "BICYCLE", held_effect: "HELD_NONE", property: "", field_menu: "", battle_menu: "" }],
        ["POKE_BALL", { name: "POKE_BALL", pocket: ItemPocket.BALL, price: 200, description: "A device for catching wild Pokémon.", effect: ItemEffect.NONE, parameter: 0, script_name: "POKE_BALL", held_effect: "HELD_NONE", property: "", field_menu: "", battle_menu: "" }],
        ["TM01", { name: "TM01", pocket: ItemPocket.TM_HM, price: 3000, description: "Teaches a move.", effect: ItemEffect.NONE, parameter: 0, script_name: "TM01", held_effect: "HELD_NONE", property: "", field_menu: "", battle_menu: "" }],
        ["HM01", { name: "HM01", pocket: ItemPocket.TM_HM, price: 0, description: "Teaches a move.", effect: ItemEffect.NONE, parameter: 0, script_name: "HM01", held_effect: "HELD_NONE", property: "", field_menu: "", battle_menu: "" }],
        // For testing canonicalisation
        ["GREAT_BALL", { name: "Great Ball", pocket: ItemPocket.BALL, price: 600, description: "A good, high-performance Ball.", effect: ItemEffect.NONE, parameter: 0, script_name: "", held_effect: "HELD_NONE", property: "", field_menu: "", battle_menu: "" }], // Missing script_name
        ["MASTER_BALL", { name: "MASTER BALL", pocket: ItemPocket.BALL, price: 0, description: "The best Ball with the ultimate performance.", effect: ItemEffect.NONE, parameter: 0, script_name: "Master Ball", held_effect: "HELD_NONE", property: "", field_menu: "", battle_menu: "" }], // Unformatted script_name
      ]),
    } as DataLoader;
    itemSystem = new ItemSystem(gameState, dataLoader);
  });

  describe("addItem", () => {
    it("rejects non-positive quantities", () => {
      expect(() => itemSystem.addItem("POTION", 0)).toThrow("quantity must be a positive integer");
      expect(() => itemSystem.addItem("POTION", -1)).toThrow("quantity must be a positive integer");
    });

    it("handles item with missing script_name by falling back to name", () => {
      expect(itemSystem.addItem("Great Ball", 1)).toBe(true);
      expect(gameState.sram.balls["GREAT_BALL"]).toBe(1);
    });

    it("handles item with unformatted script_name", () => {
        expect(itemSystem.addItem("MASTER_BALL", 1)).toBe(true);
        expect(gameState.sram.balls["MASTER_BALL"]).toBe(1);
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
        for (let i = 0; i < capacity; i++) {
            itemSystem.addItem(`ITEM_${i}`, 1);
        }
        expect(Object.keys(gameState.sram.items).length).toBe(capacity);
        expect(itemSystem.addItem("NEW_ITEM", 1)).toBe(false);
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
      expect(itemSystem.addItem("TM01", 1)).toBe(true);
      expect(itemSystem.hasItem("TM01")).toBe(true);
      expect(gameState.sram.tm_hm[0]).toBe(1);
    });

    it("adds an asm-style item alias without underscore", () => {
      expect(itemSystem.addItem("PSNCUREBERRY", 1)).toBe(true);
      expect(gameState.sram.items["PSNCUREBERRY"]).toBe(1);
      expect(itemSystem.getItemDefinition("PSNCUREBERRY").script_name).toBe("PSNCURE_BERRY");
    });

    it("adds KEY-like asm item aliases", () => {
      expect(itemSystem.addItem("BLACKBELT_I", 1)).toBe(true);
      expect(gameState.sram.items["BLACKBELT_I"]).toBe(1);
    });

    it("returns false if TM is already present", () => {
      itemSystem.addItem("TM01", 1);
      expect(itemSystem.addItem("TM01", 1)).toBe(false);
    });

    it("accepts item objects directly", () => {
      const potion: Item = {
        name: "POTION",
        pocket: ItemPocket.ITEM,
        price: 300,
        description: "Restores HP.",
        effect: ItemEffect.NONE,
        parameter: 0,
        script_name: "POTION",
        held_effect: "HELD_NONE",
        property: "",
        field_menu: "",
        battle_menu: "",
      };

      expect(itemSystem.addItem(potion, 2)).toBe(true);
      expect(itemSystem.getQuantity(potion)).toBe(2);
      expect(gameState.sram.items["POTION"]).toBe(2);
    });

    it("tracks held vs non-held metadata while sharing item inventory behavior", () => {
      const inventoryLoader = {
        itemData: new Map<string, Item>([
          [
            "CUSTOM_HELD_ITEM",
            {
              name: "CUSTOM HELD ITEM",
              pocket: ItemPocket.ITEM,
              price: 500,
              description: "An item with a held effect.",
              effect: ItemEffect.NONE,
              parameter: 0,
              script_name: "CUSTOM_HELD_ITEM",
              held_effect: "HELD_WATER_BOOST",
              property: "",
              field_menu: "",
              battle_menu: "",
            },
          ],
          [
            "CUSTOM_NON_HELD_ITEM",
            {
              name: "CUSTOM NON HELD ITEM",
              pocket: ItemPocket.ITEM,
              price: 500,
              description: "An item without a held effect.",
              effect: ItemEffect.NONE,
              parameter: 0,
              script_name: "CUSTOM_NON_HELD_ITEM",
              held_effect: "HELD_NONE",
              property: "",
              field_menu: "",
              battle_menu: "",
            },
          ],
        ]),
      } as DataLoader;
      const inventorySystem = new ItemSystem(gameState, inventoryLoader);

      expect(inventorySystem.addItem("CUSTOM_HELD_ITEM", 4)).toBe(true);
      expect(inventorySystem.getItemDefinition("CUSTOM_HELD_ITEM").held_effect).toBe("HELD_WATER_BOOST");
      expect(inventorySystem.addItem("CUSTOM_NON_HELD_ITEM", 2)).toBe(true);
      expect(inventorySystem.getItemDefinition("CUSTOM_NON_HELD_ITEM").held_effect).toBe("HELD_NONE");
      expect(inventorySystem.getQuantity("custom held item")).toBe(4);
      expect(inventorySystem.getQuantity("CUSTOM_NON_HELD_ITEM")).toBe(2);
    });

    it("supports TM/HM aliases with canonicalized item keys", () => {
      expect(itemSystem.addItem("tm 1", 1)).toBe(true);
      expect(gameState.sram.tm_hm[0]).toBe(1);
      expect(itemSystem.addItem("TM 1", 1)).toBe(false);

      expect(itemSystem.addItem("hm 1", 1)).toBe(true);
      expect(gameState.sram.tm_hm[50]).toBe(1);
    });

    it("respects pocket capacity for BALL and KEY_ITEM pockets", () => {
      const ballCapacity = POCKET_CAPACITY[ItemPocket.BALL]!;
      for (let i = 0; i < ballCapacity; i++) {
        expect(itemSystem.addItem(`BALL_SLOT_${i}_BALL`, 1)).toBe(true);
      }
      expect(Object.keys(gameState.sram.balls).length).toBe(ballCapacity);
      expect(itemSystem.addItem("SURPLUS_BALL", 1)).toBe(false);

      const keyCapacity = POCKET_CAPACITY[ItemPocket.KEY_ITEM]!;
      for (let i = 0; i < keyCapacity; i++) {
        expect(itemSystem.addItem(`KEY_SLOT_${i}_CARD`, 1)).toBe(true);
      }
      expect(Object.keys(gameState.sram.key_items).length).toBe(keyCapacity);
      expect(itemSystem.addItem("SURPLUS_CARD", 1)).toBe(false);
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
        itemSystem.addItem("TM01", 1);
        expect(itemSystem.removeItem("TM01", 1)).toBe(true);
        expect(itemSystem.hasItem("TM01")).toBe(false);
        expect(gameState.sram.tm_hm[0]).toBe(0);
    });

    it("does not remove an HM", () => {
        itemSystem.addItem("HM01", 1);
        expect(itemSystem.removeItem("HM01", 1)).toBe(false);
        expect(itemSystem.hasItem("HM01")).toBe(true);
    });

    it("removes TMs but not HMs regardless of alias form", () => {
      expect(itemSystem.removeItem("tm 1", 1)).toBe(false);

      itemSystem.addItem("tm1", 1);
      expect(itemSystem.removeItem("TM 1", 1)).toBe(true);
      expect(itemSystem.hasItem("tm 1")).toBe(false);

      itemSystem.addItem("hm 1", 1);
      expect(itemSystem.removeItem("HM 1", 1)).toBe(false);
      expect(itemSystem.hasItem("HM01")).toBe(true);
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
        itemSystem.addItem("TM01", 1);
        expect(itemSystem.getQuantity("TM01")).toBe(1);
    });
  });

  describe("lookup helpers", () => {
    it("infers pockets for item categories", () => {
      expect(itemSystem.getItemPocket("POTION")).toBe(ItemPocket.ITEM);
      expect(itemSystem.getItemPocket("POKE_BALL")).toBe(ItemPocket.BALL);
      expect(itemSystem.getItemPocket("BICYCLE")).toBe(ItemPocket.KEY_ITEM);
      expect(itemSystem.getItemPocket("TM01")).toBe(ItemPocket.TM_HM);
      expect(itemSystem.getItemPocket("HM01")).toBe(ItemPocket.TM_HM);
    });

    it("infers key-item pockets from suffixes used by asm item names", () => {
      expect(itemSystem.getItemPocket("TRAINERS_CARD")).toBe(ItemPocket.KEY_ITEM);
      expect(itemSystem.getItemPocket("RIDE_PASS")).toBe(ItemPocket.KEY_ITEM);
      expect(itemSystem.getItemPocket("MAGNET_GEAR")).toBe(ItemPocket.KEY_ITEM);
      expect(itemSystem.getItemPocket("SPECIALTICKET")).toBe(ItemPocket.KEY_ITEM);
      expect(itemSystem.getItemPocket("ULTRA_BALL")).toBe(ItemPocket.BALL);
    });

    it("classifies every content key item and common aliases as key items", () => {
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

      expect(contentSystem.getItemPocket("SQUIRTBOTTLE")).toBe(ItemPocket.KEY_ITEM);
      expect(contentSystem.getItemPocket("S.S. TICKET")).toBe(ItemPocket.KEY_ITEM);
    });

    it("formats display names for canonical, TM, HM, and fallback items", () => {
      expect(itemSystem.getDisplayName("POTION")).toBe("Potion");
      expect(itemSystem.getDisplayName("MASTER_BALL")).toBe("Master Ball");
      expect(itemSystem.getDisplayName("TM01")).toBe("TM01");
      expect(itemSystem.getDisplayName("HM01")).toBe("HM01");
      expect(itemSystem.getDisplayName("mystery_item")).toBe("Mystery Item");
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
          itemSystem.addItem("TM01", 1);
          const allItems = itemSystem.listItems();
          expect(allItems).toEqual({
              "POTION": 2,
              "BICYCLE": 1,
              "POKE_BALL": 10,
              "TM01": 1,
          });
      });
  });

  describe("fallback item definitions", () => {
    it("uses bundled content items when no data loader is provided", () => {
      const fallbackSystem = new ItemSystem(gameState);
      const potion = fallbackSystem.getItemDefinition("POTION");
      expect(potion.script_name).toBe("POTION");
      expect(potion.effect).toBe(ItemEffect.RESTORE_HP);
    });

    it("hydrates missing loader effects from bundled content data", () => {
      const potion = itemSystem.getItemDefinition("POTION");
      expect(potion.effect).toBe(ItemEffect.RESTORE_HP);
      expect(potion.parameter).toBe(0);
    });

    it("hydrates empty itemData Map loaders from bundled content", () => {
      const emptyMapLoader = { itemData: new Map<string, Item>() } as DataLoader;
      const seededSystem = new ItemSystem(gameState, emptyMapLoader);

      const elixer = seededSystem.getItemDefinition("ELIXER");
      expect(elixer.effect).toBe(ItemEffect.RESTORE_PP);
      expect((emptyMapLoader.itemData as Map<string, Item>).size).toBeGreaterThan(0);
    });

    it("hydrates empty item_data record loaders from bundled content", () => {
      const emptyRecordLoader = { item_data: {} } as DataLoader;
      const seededSystem = new ItemSystem(gameState, emptyRecordLoader);

      const elixer = seededSystem.getItemDefinition("ELIXER");
      expect(elixer.script_name).toBe("ELIXER");
      expect((emptyRecordLoader.item_data as Record<string, Item>).ELIXER).toBeDefined();
    });

    it("uses itemData Map entries directly", () => {
      const mapLoader = {
        itemData: new Map<string, Item>([
          [
            "CUSTOM_MAP_ITEM",
            {
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
            },
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
      itemSystem.addItem("tm1", 1);
      itemSystem.addItem("hm1", 1);

      expect(itemSystem.listItems(ItemPocket.TM_HM)).toEqual({
        TM01: 1,
        HM01: 1,
      });
    });
  });
});
