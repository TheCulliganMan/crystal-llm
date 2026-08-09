import { GameState } from "../../core/state";
import { DataLoader } from "../../core/data-loader";
import { Item } from "../../core/models";
import { ItemPocket } from "../../core/enums/item";
import * as tmhmSystem from "./tmhm";
import { TMHM_MOVES } from "../../core/tmhm";
import { items as contentItems } from "@pokecrystal/assets/content/items";

const CONTENT_ITEM_INDEX: Map<string, Item> = new Map();

const ensureContentItemIndex = (): Map<string, Item> => {
  if (CONTENT_ITEM_INDEX.size > 0) {
    return CONTENT_ITEM_INDEX;
  }
  for (const item of contentItems) {
    CONTENT_ITEM_INDEX.set(item.script_name, item);
  }
  return CONTENT_ITEM_INDEX;
};

const exactItemKey = (item: string | Item): string => {
  if (typeof item === "object" && item !== null) {
    return item.script_name;
  }
  return item;
};

export type ItemSystemDataLoader =
  | DataLoader
  | {
      itemData?: Map<string, Item> | Record<string, Item>;
      item_data?: Record<string, Item>;
      get_item?: (name: string) => Item | null;
    };

export const MAX_ITEM_STACK = 99;
export const POCKET_CAPACITY: Record<ItemPocket, number | null> = {
  [ItemPocket.ITEM]: 20,
  [ItemPocket.KEY_ITEM]: 25,
  [ItemPocket.BALL]: 12,
  [ItemPocket.TM_HM]: null,
};

export class ItemSystem {
  private gameState: GameState;
  private dataLoader?: ItemSystemDataLoader;

  constructor(gameState: GameState, dataLoader?: ItemSystemDataLoader) {
    this.gameState = gameState;
    this.dataLoader = dataLoader;
  }

  public addItem(item: string | Item, quantity: number = 1): boolean {
    if (quantity <= 0) {
      throw new Error("quantity must be a positive integer");
    }

    const canonical = exactItemKey(item);
    const definition = this.resolveItemDefinition(canonical);
    if (!definition) {
      throw new Error(`Unknown item definition: ${canonical}`);
    }
    const pocket = definition.pocket;

    if (pocket === ItemPocket.TM_HM) {
      return this.addTmhm(canonical);
    }

    const inventory = this.getInventoryForPocket(pocket);
    const current = inventory[canonical] || 0;
    const stackLimit = pocket === ItemPocket.KEY_ITEM ? 1 : MAX_ITEM_STACK;

    if (current >= stackLimit) {
      return false;
    }

    const capacity = POCKET_CAPACITY[pocket];
    if (current === 0 && capacity !== null) {
      const activeSlots = Object.values(inventory).filter(
        (count) => count > 0
      ).length;
      if (activeSlots >= capacity) {
        return false;
      }
    }

    const newQuantity = current + quantity;
    if (newQuantity > stackLimit) {
      return false;
    }

    if (newQuantity <= 0) {
      delete inventory[canonical];
    } else {
      inventory[canonical] = newQuantity;
    }
    return true;
  }

  public removeItem(item: string | Item, quantity: number = 1): boolean {
    if (quantity <= 0) {
      throw new Error("quantity must be a positive integer");
    }

    const canonical = exactItemKey(item);
    const definition = this.resolveRequiredItemDefinition(canonical);
    if (definition.pocket === ItemPocket.TM_HM) {
      return this.removeTmhm(canonical);
    }

    return this.decrementFromInventory(this.getInventoryForPocket(definition.pocket), canonical, quantity);
  }

  public hasItem(item: string | Item): boolean {
    const canonical = exactItemKey(item);
    const definition = this.resolveRequiredItemDefinition(canonical);
    if (definition.pocket === ItemPocket.TM_HM) {
      return this.tmhmHas(canonical);
    }
    return (this.getInventoryForPocket(definition.pocket)[canonical] || 0) > 0;
  }

  public getQuantity(item: string | Item): number {
    const canonical = exactItemKey(item);
    const definition = this.resolveRequiredItemDefinition(canonical);
    if (definition.pocket === ItemPocket.TM_HM) {
      return this.tmhmHas(canonical) ? 1 : 0;
    }
    return this.getInventoryForPocket(definition.pocket)[canonical] || 0;
  }

  public listItems(pocket?: ItemPocket): Record<string, number> {
    if (pocket === undefined) {
      const combined: Record<string, number> = {};
      for (const [pocketId, inventory] of this.allPockets()) {
        if (pocketId === ItemPocket.TM_HM) {
          Object.assign(combined, this.tmhmInventoryView());
          continue;
        }
        for (const [name, quantity] of Object.entries(inventory)) {
          combined[name] = (combined[name] || 0) + quantity;
        }
      }
      return combined;
    }
    return { ...this.getInventoryForPocket(pocket) };
  }

  public getItemDefinition(item: string | Item): Item {
    const canonical = exactItemKey(item);
    const definition = this.resolveItemDefinition(canonical);
    if (definition) {
      return definition;
    }
    throw new Error(`Unknown item definition: ${canonical}`);
  }

  public getDisplayName(item: string | Item): string {
    const definition = this.resolveRequiredItemDefinition(exactItemKey(item));
    let baseName = definition.name;

    baseName = baseName.replace(/_/g, " ").trim();
    if (baseName.startsWith("TM") && /^\d+$/.test(baseName.substring(2).trim())) {
      return baseName.replace(/ /g, "");
    }
    if (baseName.startsWith("HM") && /^\d+$/.test(baseName.substring(2).trim())) {
      return baseName.replace(/ /g, "");
    }
    if (baseName.startsWith("TM ") && baseName.substring(3)) {
      return "TM " + this.titleCase(baseName.substring(3));
    }
    if (baseName.startsWith("HM ") && baseName.substring(3)) {
      return "HM " + this.titleCase(baseName.substring(3));
    }
    if (baseName === baseName.toUpperCase()) {
      return this.titleCase(baseName);
    }
    return baseName;
  }

  public getItemPocket(item: string | Item): ItemPocket {
    const canonical = exactItemKey(item);
    const definition = this.resolveItemDefinition(canonical);
    if (!definition) {
      throw new Error(`Unknown item definition: ${canonical}`);
    }
    return definition.pocket;
  }

  private titleCase(s: string): string {
    return s.replace(/\w\S*/g, (txt) => txt.charAt(0).toUpperCase() + txt.substring(1).toLowerCase());
  }

  private allPockets(): [ItemPocket, Record<string, number>][] {
    return [
      [ItemPocket.ITEM, this.gameState.sram.items],
      [ItemPocket.KEY_ITEM, this.gameState.sram.key_items],
      [ItemPocket.BALL, this.gameState.sram.balls],
      [ItemPocket.TM_HM, this.tmhmInventoryView()],
    ];
  }

  private getInventoryForPocket(pocket: ItemPocket): Record<string, number> {
    const pocketMap: Record<string, Record<string, number>> = {
      [ItemPocket.ITEM]: this.gameState.sram.items,
      [ItemPocket.KEY_ITEM]: this.gameState.sram.key_items,
      [ItemPocket.BALL]: this.gameState.sram.balls,
    };
    if (pocket === ItemPocket.TM_HM) {
      return this.tmhmInventoryView();
    }
    if (!pocketMap[pocket]) {
      throw new Error(`Unsupported pocket: ${pocket}`);
    }
    return pocketMap[pocket];
  }

  private getItemDataCollection(): Map<string, Item> | Record<string, Item> | undefined {
    const loader = this.dataLoader;
    if (!loader) {
      return undefined;
    }
    if ("itemData" in loader && loader.itemData) {
      if (loader.itemData instanceof Map) {
        return loader.itemData.size > 0 ? loader.itemData : undefined;
      }
      return Object.keys(loader.itemData).length > 0 ? loader.itemData : undefined;
    }
    if ("item_data" in loader) {
      if (loader.item_data instanceof Map) {
        return loader.item_data.size > 0 ? loader.item_data : undefined;
      }
      return loader.item_data && Object.keys(loader.item_data).length > 0
        ? loader.item_data
        : undefined;
    }
    return undefined;
  }

  private resolveItemDefinition(canonical: string): Item | undefined {
    const itemData = this.getItemDataCollection();
    if (itemData instanceof Map) {
      const definition = itemData.get(canonical);
      if (definition) {
        return definition;
      }
    } else if (itemData?.[canonical]) {
      return itemData[canonical];
    }
    const loaded = this.dataLoader?.get_item?.(canonical) ?? null;
    if (loaded) {
      return loaded;
    }
    if (this.dataLoader) {
      return undefined;
    }
    return this.resolveContentDefinition(canonical);
  }

  private resolveRequiredItemDefinition(canonical: string): Item {
    const definition = this.resolveItemDefinition(canonical);
    if (!definition) {
      throw new Error(`Unknown item definition: ${canonical}`);
    }
    return definition;
  }

  private resolveContentDefinition(canonical: string): Item | undefined {
    return ensureContentItemIndex().get(canonical);
  }

  private tmhmFlags(): number[] {
    const flags = this.gameState.sram.tm_hm;
    const required = TMHM_MOVES.length;
    if (flags.length < required) {
      flags.push(...Array(required - flags.length).fill(0));
    }
    return flags;
  }

  private tmhmInventoryView(): Record<string, number> {
    const flags = this.tmhmFlags();
    const inventory: Record<string, number> = {};
    flags.forEach((enabled, index) => {
      if (enabled) {
        inventory[tmhmSystem.tmhmItemName(index)] = 1;
      }
    });
    return inventory;
  }

  private tmhmHas(canonical: string): boolean {
    const index = tmhmSystem.tmhmIndex(canonical);
    const flags = this.tmhmFlags();
    if (index < 0 || index >= flags.length) {
      return false;
    }
    return !!flags[index];
  }

  private addTmhm(canonical: string): boolean {
    const index = tmhmSystem.tmhmIndex(canonical);
    const flags = this.tmhmFlags();
    if (flags[index]) {
      return false; // Already have it
    }
    flags[index] = 1;
    return true;
  }

  private removeTmhm(canonical: string, quantity: number = 1): boolean {
    if (quantity <= 0) {
      throw new Error("quantity must be a positive integer");
    }
    const index = tmhmSystem.tmhmIndex(canonical);
    const flags = this.tmhmFlags();
    if (tmhmSystem.isHmIndex(index)) {
      return false; // Can't remove HMs
    }
    if (!flags[index]) {
      return false; // Don't have it
    }
    flags[index] = 0;
    return true;
  }

  private decrementFromInventory(
    inventory: Record<string, number>,
    canonical: string,
    quantity: number
  ): boolean {
    const current = inventory[canonical];
    if (current === undefined) {
      return false;
    }
    if (current <= quantity) {
      delete inventory[canonical];
    } else {
      inventory[canonical] = current - quantity;
    }
    return true;
  }

}
