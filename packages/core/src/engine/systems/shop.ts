import { z } from "zod";
import { GameState } from "../../core/state";
import { DataLoader } from "../../core/data-loader";
import { ItemSystem, MAX_ITEM_STACK, POCKET_CAPACITY } from "./items";
import { ItemPocket } from "../../core/enums/item";
import { MAX_MONEY } from "../../core/constants";

const PRICE_DIGITS = 6;

export const formatPrice = (value: number): string => {
  const clamped = Math.max(0, Math.min(MAX_MONEY, Math.floor(value)));
  return `¥${clamped.toString().padStart(PRICE_DIGITS, "0")}`;
};

export const MartItemSchema = z.object({
  identifier: z.string(),
  displayName: z.string(),
  price: z.number(),
  quantity: z.number().optional(),
});

export type MartItem = z.infer<typeof MartItemSchema>;

export const ShopResultSchema = z.object({
  success: z.boolean(),
  message: z.string(),
  credited: z.number().default(0),
});

export type ShopResult = z.infer<typeof ShopResultSchema>;

const normalizeMartKey = (martIdentifier: string): string => martIdentifier.trim().toUpperCase();

const exportedMartKeyAlias = (martIdentifier: string): string => normalizeMartKey(martIdentifier).replace(/_/g, "");

export class Shop {
  private gameState: GameState;
  private itemSystem: ItemSystem;
  private dataLoader: DataLoader;

  constructor(
    gameState: GameState,
    itemSystem: ItemSystem,
    dataLoader: DataLoader
  ) {
    this.gameState = gameState;
    this.itemSystem = itemSystem;
    this.dataLoader = dataLoader;
  }

  public loadInventory(martIdentifier: string): MartItem[] {
    const martKey = normalizeMartKey(martIdentifier);
    if (!this.dataLoader.martData) {
      this.dataLoader.loadMartData();
    }
    const itemNames =
      this.dataLoader.martData?.get(martKey) ??
      this.dataLoader.martData?.get(exportedMartKeyAlias(martIdentifier)) ??
      [];
    const items: MartItem[] = [];
    for (const name of itemNames) {
      const definition = this.itemSystem.getItemDefinition(name);
      const displayName = this.itemSystem.getDisplayName(name);
      const price = definition?.price ?? 0;
      items.push({ identifier: name, displayName, price });
    }
    return items;
  }

  public buildBuyMenu(martIdentifier: string): MartItem[] {
    const items = this.loadInventory(martIdentifier);
    items.push({ identifier: "CANCEL", displayName: "CANCEL", price: 0 });
    return items;
  }

  public maxBuyQuantity(item: MartItem): number {
    if (item.price <= 0) {
      return 0;
    }
    const itemPocket = this.itemSystem.getItemPocket(item.identifier);
    const pocketCapacity = POCKET_CAPACITY[itemPocket];
    const owned = this.itemSystem.getQuantity(item.identifier);

    if (owned === 0 && pocketCapacity !== null) {
      const uniqueItemsInPocket = Object.keys(
        this.itemSystem.listItems(itemPocket)
      ).length;
      if (uniqueItemsInPocket >= pocketCapacity) {
        return 0;
      }
    }

    const stackLimit =
      itemPocket === ItemPocket.KEY_ITEM ? 1 : MAX_ITEM_STACK;
    const capacity = Math.max(0, stackLimit - owned);
    const affordable = Math.floor(this.gameState.sram.money / item.price);
    return Math.min(capacity, affordable);
  }

  public buyItem(item: MartItem, quantity: number): ShopResult {
    if (quantity <= 0) {
      return {
        success: false,
        message: "Quantity must be positive.",
        credited: 0,
      };
    }

    const totalCost = item.price * quantity;
    if (totalCost > this.gameState.sram.money) {
      return {
        success: false,
        message: "You don't have enough money.",
        credited: 0,
      };
    }

    if (!this.itemSystem.addItem(item.identifier, quantity)) {
      return { success: false, message: "Your Pack is full.", credited: 0 };
    }

    this.gameState.sram.money -= totalCost;
    return {
      success: true,
      message: formatPrice(totalCost),
      credited: totalCost,
    };
  }

  public sellItem(item: MartItem, quantity: number): ShopResult {
    const sellPrice = Math.max(0, Math.floor(item.price / 2));

    if (quantity <= 0) {
      return {
        success: false,
        message: "Quantity must be positive.",
        credited: 0,
      };
    }
    if (sellPrice <= 0) {
      return {
        success: false,
        message: "We can't offer anything for that item.",
        credited: 0,
      };
    }
    const owned = this.itemSystem.getQuantity(item.identifier);
    if (owned < quantity) {
      return {
        success: false,
        message: "Looks like you don't have that many.",
        credited: 0,
      };
    }
    if (!this.itemSystem.removeItem(item.identifier, quantity)) {
      return {
        success: false,
        message: "Looks like you don't have that many.",
        credited: 0,
      };
    }

    const payout = sellPrice * quantity;
    const startingMoney = this.gameState.sram.money;
    const newMoney = Math.min(MAX_MONEY, startingMoney + payout);
    this.gameState.sram.money = newMoney;
    const credited = newMoney - startingMoney;
    return {
      success: true,
      message: formatPrice(payout),
      credited: credited,
    };
  }
}

const MART_MENU_PAGE_SIZE = 4;

export const paginateSelection = (
  selection: number,
  scroll: number,
  totalItems: number,
  direction: "up" | "down"
): [number, number] => {
  const page = MART_MENU_PAGE_SIZE;
  if (direction === "up") {
    selection = Math.max(0, selection - 1);
  } else if (direction === "down") {
    selection = Math.min(totalItems - 1, selection + 1);
  }

  if (selection < scroll) {
    scroll = selection;
  } else if (selection >= scroll + page) {
    scroll = selection - page + 1;
  }
  return [selection, scroll];
};
