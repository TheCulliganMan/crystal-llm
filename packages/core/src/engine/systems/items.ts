import { GameState } from "../../core/state";
import { DataLoader } from "../../core/data-loader";
import { Item } from "../../core/models";
import { ItemEffect, ItemPocket } from "../../core/enums/item";
import * as tmhmSystem from "./tmhm";
import { TMHM_MOVES } from "../../core/tmhm";
import { items as contentItems } from "@pokecrystal/assets/content/items";

const CONTENT_ITEM_INDEX: Map<string, Item> = new Map();

const normalizeItemLookupKey = (value: string): string => {
  return value
    .normalize("NFKD")
    .toUpperCase()
    .replace(/[^A-Z0-9]/g, "");
};

const addItemIndexAlias = (key: string, item: Item): void => {
  if (!CONTENT_ITEM_INDEX.has(key)) {
    CONTENT_ITEM_INDEX.set(key, item);
  }

  const normalized = normalizeItemLookupKey(key);
  if (normalized && !CONTENT_ITEM_INDEX.has(normalized)) {
    CONTENT_ITEM_INDEX.set(normalized, item);
  }

  const scriptNameAlias = normalizeItemLookupKey(item.script_name ?? item.name);
  if (scriptNameAlias && !CONTENT_ITEM_INDEX.has(scriptNameAlias)) {
    CONTENT_ITEM_INDEX.set(scriptNameAlias, item);
  }
};

const ensureContentItemIndex = (): Map<string, Item> => {
  if (CONTENT_ITEM_INDEX.size > 0) {
    return CONTENT_ITEM_INDEX;
  }
  for (const item of contentItems) {
    addItemIndexAlias(item.script_name, item);
  }
  return CONTENT_ITEM_INDEX;
};

const canonicaliseItemName = (item: string | Item): string => {
  let nameToProcess: string;
  if (typeof item === "object" && item !== null) {
    nameToProcess = item.script_name || item.name;
  } else {
    nameToProcess = item as string;
  }
  return nameToProcess.replace(/ /g, "_").toUpperCase();
};

export type ItemSystemDataLoader =
  | DataLoader
  | {
      itemData?: Map<string, Item> | Record<string, Item>;
      item_data?: Record<string, Item>;
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

    const canonical = canonicaliseItemName(item);
    const definition = this.resolveItemDefinition(canonical);
    const pocket = this.inferPocket(canonical, definition);

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

    const canonical = canonicaliseItemName(item);
    if (
      this.inferPocket(canonical, this.resolveItemDefinition(canonical)) ===
      ItemPocket.TM_HM
    ) {
      return this.removeTmhm(canonical);
    }

    const definition = this.resolveItemDefinition(canonical);
    const pocket = this.inferPocket(canonical, definition);
    const inventory = this.getInventoryForPocket(pocket);
    if (this.decrementFromInventory(inventory, canonical, quantity)) {
      return true;
    }

    for (const [, fallbackInventory] of this.allPockets()) {
      if (fallbackInventory === inventory) {
        continue;
      }
      if (
        this.decrementFromInventory(
          fallbackInventory as Record<string, number>,
          canonical,
          quantity
        )
      ) {
        return true;
      }
    }
    return false;
  }

  public hasItem(item: string | Item): boolean {
    const canonical = canonicaliseItemName(item);
    if (
      this.inferPocket(canonical, this.resolveItemDefinition(canonical)) ===
      ItemPocket.TM_HM
    ) {
      return this.tmhmHas(canonical);
    }
    for (const [, inventory] of this.allPockets()) {
      if ((inventory[canonical] || 0) > 0) {
        return true;
      }
    }
    return false;
  }

  public getQuantity(item: string | Item): number {
    const canonical = canonicaliseItemName(item);
    if (
      this.inferPocket(canonical, this.resolveItemDefinition(canonical)) ===
      ItemPocket.TM_HM
    ) {
      return this.tmhmHas(canonical) ? 1 : 0;
    }
    let total = 0;
    for (const [, inventory] of this.allPockets()) {
      total += inventory[canonical] || 0;
    }
    return total;
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
    const canonical = canonicaliseItemName(item);
    const definition = this.resolveItemDefinition(canonical);
    if (definition) {
      return definition;
    }
    // Fallback
    const displayName = canonical.replace(/_/g, " ");
    return {
      name: displayName,
      description: "No description available.",
      price: 0,
      pocket: ItemPocket.ITEM,
      parameter: 0,
      effect: ItemEffect.NONE,
      script_name: canonical,
      held_effect: "HELD_NONE",
      property: "",
      field_menu: "",
      battle_menu: "",
    };
  }

  public getDisplayName(item: string | Item): string {
    const definition = this.resolveItemDefinition(canonicaliseItemName(item));
    let baseName: string;
    if (!definition) {
      baseName = canonicaliseItemName(item).replace(/_/g, " ");
    } else {
      baseName = definition.name;
    }

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
    const canonical = canonicaliseItemName(item);
    const definition = this.resolveItemDefinition(canonical);
    return this.inferPocket(canonical, definition);
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
      return loader.itemData;
    }
    if ("item_data" in loader) {
      return loader.item_data;
    }
    return undefined;
  }

  private resolveItemDefinition(canonical: string): Item | undefined {
    const itemData = this.getItemDataCollection();
    if (!itemData) {
      return this.resolveContentDefinition(canonical);
    }
    const candidates = this.candidateLookupNames(canonical);
    if (itemData instanceof Map) {
      if (itemData.size === 0) {
        this.populateItemData(itemData);
      }
      return this.hydrateDefinition(canonical, this.lookupInItemCollection(itemData, candidates));
    }
    if (Object.keys(itemData).length === 0) {
      const seeded = this.seedItemRecord();
      for (const [key, value] of Object.entries(seeded)) {
        itemData[key] = value;
      }
    }
    return this.hydrateDefinition(canonical, this.lookupInItemCollection(itemData, candidates));
  }

  private resolveContentDefinition(canonical: string): Item | undefined {
    return this.lookupInItemCollection(ensureContentItemIndex(), this.candidateLookupNames(canonical));
  }

  private hydrateDefinition(canonical: string, definition: Item | undefined): Item | undefined {
    if (!definition) {
      return this.resolveContentDefinition(canonical);
    }
    if (definition.effect !== ItemEffect.NONE) {
      return definition;
    }
    const contentDefinition = this.resolveContentDefinition(canonical);
    if (!contentDefinition || contentDefinition.effect === ItemEffect.NONE) {
      return definition;
    }
    return {
      ...contentDefinition,
      ...definition,
      effect: contentDefinition.effect,
    };
  }

  private candidateLookupNames(canonical: string): string[] {
    const base = canonical.toUpperCase();
    const withSpaces = base.replace(/_/g, " ");
    const variants = new Set<string>([canonical, base, withSpaces]);
    const titleVariant = this.titleCase(withSpaces);
    if (!variants.has(titleVariant)) {
      variants.add(titleVariant);
    }
    const collapsed = withSpaces.replace(/ /g, "");
    if (!variants.has(collapsed)) {
      variants.add(collapsed);
    }
    const normalized = normalizeItemLookupKey(base);
    if (normalized) {
      variants.add(normalized);
    }
    const normalizedTitle = normalizeItemLookupKey(titleVariant);
    if (normalizedTitle) {
      variants.add(normalizedTitle);
    }
    if (canonical !== base) {
      const canonicalNormalized = normalizeItemLookupKey(canonical);
      if (canonicalNormalized) {
        variants.add(canonicalNormalized);
      }
    }
    return [...variants];
  }

  private lookupInItemCollection(
    itemData: Map<string, Item> | Record<string, Item>,
    candidates: string[],
  ): Item | undefined {
    for (const candidate of candidates) {
      if (itemData instanceof Map) {
        const direct = itemData.get(candidate);
        if (direct) {
          return direct;
        }
      } else {
        const direct = itemData[candidate];
        if (direct) {
          return direct;
        }
      }
    }

    const normalizedCandidates = new Set(
      candidates.map((candidate) => normalizeItemLookupKey(candidate))
    );

    if (itemData instanceof Map) {
      for (const [key, item] of itemData.entries()) {
        const itemScriptName = item.script_name || "";
        const itemName = item.name || "";
        const keys = [key, itemScriptName, itemName];
        for (const candidateKey of keys) {
          if (normalizedCandidates.has(normalizeItemLookupKey(candidateKey))) {
            return item;
          }
        }
      }
      return undefined;
    }

    for (const [key, item] of Object.entries(itemData)) {
      const itemScriptName = item.script_name || "";
      const itemName = item.name || "";
      const keys = [key, itemScriptName, itemName];
      for (const candidateKey of keys) {
        if (normalizedCandidates.has(normalizeItemLookupKey(candidateKey))) {
          return item;
        }
      }
    }
    return undefined;
  }

  private inferPocket(canonical: string, definition?: Item): ItemPocket {
    if (definition?.pocket) {
      return definition.pocket;
    }
    const upperName = canonical.toUpperCase();
    if (upperName.startsWith("TM") || upperName.startsWith("HM")) {
      return ItemPocket.TM_HM;
    }
    if (upperName.endsWith("_BALL") || upperName.endsWith("BALL")) {
      return ItemPocket.BALL;
    }
    if (
      upperName.endsWith("_CARD") ||
      upperName.endsWith("_PASS") ||
      upperName.endsWith("TICKET") ||
      upperName.endsWith("GEAR")
    ) {
      return ItemPocket.KEY_ITEM;
    }
    return ItemPocket.ITEM;
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

  private populateItemData(itemData: Map<string, Item>): void {
    for (const item of contentItems) {
      itemData.set(item.script_name, item);
      itemData.set(item.name, item);
      const normalized = normalizeItemLookupKey(item.script_name);
      if (normalized) {
        itemData.set(normalized, item);
      }
    }
  }

  private seedItemRecord(): Record<string, Item> {
    const record: Record<string, Item> = {};
    for (const item of contentItems) {
      record[item.script_name] = item;
      record[item.name] = item;
      const normalized = normalizeItemLookupKey(item.script_name);
      if (normalized) {
        record[normalized] = item;
      }
    }
    return record;
  }
}
