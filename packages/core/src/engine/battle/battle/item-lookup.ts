import { loadAllItems } from "@pokecrystal/core/core/data-loader";
import type { Item } from "@pokecrystal/core/core/models";

let itemsMap: Map<string, Item> | null = null;

export function getBattleItemsMap(): Map<string, Item> {
  if (itemsMap !== null) {
    return itemsMap;
  }

  const loadedItems = loadAllItems();
  const map = new Map<string, Item>();
  for (const item of loadedItems.values()) {
    if (item.name) {
      map.set(item.name, item);
      map.set(item.name.replace(/ /g, "_"), item);
    }
    const scriptName = item.script_name || item.name;
    if (scriptName) {
      map.set(scriptName, item);
    }
  }

  itemsMap = map;
  return itemsMap;
}

export function getBattleItem(itemName?: string | null): Item | undefined {
  if (!itemName) {
    return undefined;
  }
  const items = getBattleItemsMap();
  return items.get(itemName) || items.get(itemName.replace(/ /g, "_"));
}
