// ASM mapping: pokecrystal_disassembly/engine/menus/mart.asm (buy/sell menu flow).
import { DataLoader } from "@pokecrystal/core/core/data-loader";
import { GameState } from "@pokecrystal/core/core/state";
import { ItemPocket } from "@pokecrystal/core/core/enums/item";
import { ItemSystem } from "@pokecrystal/core/engine/systems/items";
import { MartItem, Shop, formatPrice, paginateSelection } from "@pokecrystal/core/engine/systems/shop";
import {
  GameButton,
  buttonKeys,
  isKeyDownEvent,
  normalizeButtonKey,
  type KeyEvent,
} from "@pokecrystal/core/input/buttons";
import { mapKeyToDirection } from "@pokecrystal/core/input/controls";
import { Surface } from "@pokecrystal/core/ui/surface";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import { SCREEN_TILE_HEIGHT, SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";
import { renderTextSnapshot } from "@pokecrystal/core/ui/text-overlays";

const TILE_SIZE = 8;
const MART_MENU_PAGE_SIZE = 4;
const MART_ITEM_ROW_HEIGHT_TILES = 2;
const MART_ITEM_LIST_HEIGHT_TILES = MART_MENU_PAGE_SIZE * MART_ITEM_ROW_HEIGHT_TILES + 2;
const BUY_PRICE_OFFSET_TILES = 8;
const MART_VIEWPORT_TITLE = "Mart";
const MART_INFO_TITLE = "Legend";
const MART_MENU_CONTROLS = ["D-Pad=Move A=Select B=Back"];
const MART_PROMPT_CONTROLS = ["Up/Down=Qty A=OK B=Cancel"];
const MART_DIALOGUE_CONTROLS = ["A/B=Continue"];
const MART_TEXT = {
  welcome: "Welcome! How may I\nhelp you?",
  askMore: "Can I do anything\nelse for you?",
  comeAgain: "Please come again!",
  soldOut: "Sorry, we're sold out.",
  emptyShelves: "The shelves are empty right now.",
  thanks: "Here you are.\nThank you!",
  noMoney: "You don't have\nenough money.",
  bagFull: "You can't carry\nany more items.",
};

export interface MartUI {
  screen: Surface | null;
  screenHeight: number;
  drawWindow: (
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    options?: { fill?: [number, number, number] | null; zIndex?: number }
  ) => void;
  font: {
    renderText: (
      text: string,
      x: number,
      y: number,
      surface: Surface,
      options?: { textWidth?: number; maxLines?: number; uppercase?: boolean }
    ) => void;
  };
  update: () => void;
  eventQueue?: GameEngineEventQueue;
  pollEvents?: () => KeyEvent[];
  tileSize?: number;
  tile_size?: number;
  renderSnapshot?: (
    viewportLines: string[],
    infoLines: string[],
    viewportTitle?: string,
    infoTitle?: string,
    menuLines?: string[] | null,
    promptLines?: string[] | null,
    dialogueLines?: string[] | null
  ) => void;
}

interface MartWindow {
  x: number;
  y: number;
  widthTiles: number;
  heightTiles: number;
  toPixels: (tileSize: number) => [number, number, number, number];
}

const createWindow = (x: number, y: number, widthTiles: number, heightTiles: number): MartWindow => ({
  x,
  y,
  widthTiles,
  heightTiles,
  toPixels: (tileSize: number) => {
    if (tileSize <= 0) {
      throw new Error("tileSize must be positive to compute mart window bounds");
    }
    return [x * tileSize, y * tileSize, widthTiles, heightTiles];
  },
});

const MART_LAYOUT = {
  topMenu: createWindow(0, 0, 8, 9),
  itemList: createWindow(1, 3, SCREEN_TILE_WIDTH - 1, MART_ITEM_LIST_HEIGHT_TILES),
  moneyWindow: createWindow(11, 0, SCREEN_TILE_WIDTH - 11, 3),
  quantityPrompt: createWindow(7, 15, SCREEN_TILE_WIDTH - 7, SCREEN_TILE_HEIGHT - 15),
  descriptionBox: createWindow(0, TEXTBOX_Y_TILES, SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES),
};

const isButtonEvent = (key: string | number | null | undefined, button: GameButton): boolean => {
  const keyCode = normalizeButtonKey(key ?? null);
  if (keyCode === null) {
    return false;
  }
  return buttonKeys[button].includes(keyCode);
};

export class MartInterface {
  private shop: Shop;
  private itemSystem: ItemSystem;
  private readonly tileSize: number;
  private activeFlag = false;

  constructor(
    private readonly overworld: {
      ui?: MartUI;
      pollEvents?: () => KeyEvent[];
      draw?: () => void;
      dialogue?: { suspend?: () => void; resume?: () => void; _suspended?: boolean };
      input_capture_active?: boolean;
    },
    private gameState: GameState,
    private dataLoader: DataLoader,
    itemSystem: ItemSystem
  ) {
    this.itemSystem = itemSystem;
    this.shop = new Shop(gameState, itemSystem, dataLoader);
    const ui = this.overworld.ui;
    this.tileSize = ui?.tileSize ?? ui?.tile_size ?? TILE_SIZE;
  }

  public updateContext(gameState: GameState, dataLoader: DataLoader, itemSystem: ItemSystem): void {
    this.gameState = gameState;
    this.dataLoader = dataLoader;
    this.itemSystem = itemSystem;
    this.shop = new Shop(gameState, itemSystem, dataLoader);
  }

  private iterEvents(): KeyEvent[] {
    const overworldPoll = this.overworld.pollEvents;
    if (typeof overworldPoll === "function") {
      return overworldPoll();
    }
    const ui = this.overworld.ui;
    if (ui && typeof ui.pollEvents === "function") {
      return ui.pollEvents();
    }
    if (ui?.eventQueue) {
      return gameEngine.event.get(ui.eventQueue);
    }
    return [];
  }

  private captureInput(): () => void {
    if (!this.overworld) {
      return () => undefined;
    }
    const previous = Boolean(this.overworld.input_capture_active);
    this.overworld.input_capture_active = true;
    return () => {
      this.overworld.input_capture_active = previous;
    };
  }

  open(martType: string, martIdentifier: string): void {
    this.activeFlag = true;
    const restoreInput = this.captureInput();
    const restoreDialogue = this.suppressScriptDialogue();
    try {
      const ui = this.overworld.ui;
      if (!ui) {
        throw new Error("Overworld must expose a UI instance for the mart interface");
      }
      martType = martType.toUpperCase();
      if (martType !== "MARTTYPE_STANDARD") {
        this.showMessage(ui, "The clerk can't help with that request yet.");
        return;
      }
      const inventory = this.shop.loadInventory(martIdentifier);
      if (!inventory.length) {
        this.showMessage(ui, MART_TEXT.emptyShelves);
        return;
      }
      this.showMessage(ui, MART_TEXT.welcome);
      const options = ["BUY", "SELL", "QUIT"];
      let selection = 0;
      let running = true;
      while (running) {
        this.drawBase();
        this.drawMoney(ui);
        this.drawTopMenu(ui, options, selection);
        this.renderTopMenuSnapshot(ui, options, selection);
        this.present(ui);
        for (const event of this.iterEvents()) {
          if (event.type === "quit") {
            throw new Error("Quit requested while mart interface active.");
          }
          if (!isKeyDownEvent(event)) {
            continue;
          }
          const key = event.code ?? event.key ?? "";
          const direction = mapKeyToDirection(key ?? null);
          if (direction === "up") {
            selection = (selection - 1 + options.length) % options.length;
          } else if (direction === "down") {
            selection = (selection + 1) % options.length;
          } else if (isButtonEvent(key, GameButton.B)) {
            running = false;
            break;
          } else if (isButtonEvent(key, GameButton.A)) {
            const choice = options[selection];
            if (choice === "BUY") {
              this.runBuyFlow(ui, martIdentifier);
              this.showMessage(ui, MART_TEXT.askMore);
            } else if (choice === "SELL") {
              this.runSellFlow(ui);
              this.showMessage(ui, MART_TEXT.askMore);
            } else {
              running = false;
              break;
            }
          }
        }
      }
      this.showMessage(ui, MART_TEXT.comeAgain);
    } finally {
      this.activeFlag = false;
      restoreInput();
      restoreDialogue();
    }
  }

  async openAsync(martType: string, martIdentifier: string): Promise<void> {
    this.activeFlag = true;
    const restoreInput = this.captureInput();
    const restoreDialogue = this.suppressScriptDialogue();
    try {
      const ui = this.overworld.ui;
      if (!ui) {
        throw new Error("Overworld must expose a UI instance for the mart interface");
      }
      martType = martType.toUpperCase();
      if (martType !== "MARTTYPE_STANDARD") {
        await this.showMessageAsync(ui, "The clerk can't help with that request yet.");
        return;
      }
      const inventory = this.shop.loadInventory(martIdentifier);
      if (!inventory.length) {
        await this.showMessageAsync(ui, MART_TEXT.emptyShelves);
        return;
      }
      await this.showMessageAsync(ui, MART_TEXT.welcome);
      const options = ["BUY", "SELL", "QUIT"];
      let selection = 0;
      let running = true;
      while (running) {
        this.drawBase();
        this.drawMoney(ui);
        this.drawTopMenu(ui, options, selection);
        this.renderTopMenuSnapshot(ui, options, selection);
        this.present(ui);
        for (const event of this.iterEvents()) {
          if (event.type === "quit") {
            throw new Error("Quit requested while mart interface active.");
          }
          if (!isKeyDownEvent(event)) {
            continue;
          }
          const key = event.code ?? event.key ?? "";
          const direction = mapKeyToDirection(key ?? null);
          if (direction === "up") {
            selection = (selection - 1 + options.length) % options.length;
          } else if (direction === "down") {
            selection = (selection + 1) % options.length;
          } else if (isButtonEvent(key, GameButton.B)) {
            running = false;
            break;
          } else if (isButtonEvent(key, GameButton.A)) {
            const choice = options[selection];
            if (choice === "BUY") {
              await this.runBuyFlowAsync(ui, martIdentifier);
              await this.showMessageAsync(ui, MART_TEXT.askMore);
            } else if (choice === "SELL") {
              await this.runSellFlowAsync(ui);
              await this.showMessageAsync(ui, MART_TEXT.askMore);
            } else {
              running = false;
              break;
            }
          }
        }
        if (running) {
          await nextFrame();
        }
      }
      await this.showMessageAsync(ui, MART_TEXT.comeAgain);
    } finally {
      this.activeFlag = false;
      restoreInput();
      restoreDialogue();
    }
  }

  get active(): boolean {
    return this.activeFlag;
  }

  private runBuyFlow(ui: MartUI, martIdentifier: string): void {
    const items = this.shop.buildBuyMenu(martIdentifier);
    if (!items.some((item) => item.identifier !== "CANCEL")) {
      this.showMessage(ui, MART_TEXT.soldOut);
      return;
    }
    let selection = 0;
    let scroll = 0;
    while (true) {
      this.drawBase();
      this.drawMoney(ui);
      this.drawItemList(ui, items, selection, scroll, "buy");
      this.renderBuyMenuSnapshot(ui, items, selection, scroll);
      this.present(ui);
      for (const event of this.iterEvents()) {
        if (event.type === "quit") {
          throw new Error("Quit requested while mart interface active.");
        }
        if (!isKeyDownEvent(event)) {
          continue;
        }
        const key = event.code ?? event.key ?? "";
        const direction = mapKeyToDirection(key ?? null);
        if (direction === "up") {
          [selection, scroll] = paginateSelection(selection, scroll, items.length, "up");
        } else if (direction === "down") {
          [selection, scroll] = paginateSelection(selection, scroll, items.length, "down");
        } else if (isButtonEvent(key, GameButton.B)) {
          return;
        } else if (isButtonEvent(key, GameButton.A)) {
          const selectedItem = items[selection];
          if (selectedItem.identifier === "CANCEL") {
            return;
          }
          if (selectedItem.price <= 0) {
            this.showMessage(ui, "That item isn't for sale right now.");
            break;
          }
          const quantity = this.promptQuantity(ui, selectedItem, "buy");
          if (quantity === null) {
            break;
          }
          const purchase = this.shop.buyItem(selectedItem, quantity);
          if (!purchase.success) {
            this.showMessage(ui, purchase.message);
            break;
          }
          this.showMessage(ui, MART_TEXT.thanks);
          break;
        }
      }
    }
  }

  private async runBuyFlowAsync(ui: MartUI, martIdentifier: string): Promise<void> {
    const items = this.shop.buildBuyMenu(martIdentifier);
    if (!items.some((item) => item.identifier !== "CANCEL")) {
      await this.showMessageAsync(ui, MART_TEXT.soldOut);
      return;
    }
    let selection = 0;
    let scroll = 0;
    while (true) {
      this.drawBase();
      this.drawMoney(ui);
      this.drawItemList(ui, items, selection, scroll, "buy");
      this.renderBuyMenuSnapshot(ui, items, selection, scroll);
      this.present(ui);
      for (const event of this.iterEvents()) {
        if (event.type === "quit") {
          throw new Error("Quit requested while mart interface active.");
        }
        if (!isKeyDownEvent(event)) {
          continue;
        }
        const key = event.code ?? event.key ?? "";
        const direction = mapKeyToDirection(key ?? null);
        if (direction === "up") {
          [selection, scroll] = paginateSelection(selection, scroll, items.length, "up");
        } else if (direction === "down") {
          [selection, scroll] = paginateSelection(selection, scroll, items.length, "down");
        } else if (isButtonEvent(key, GameButton.B)) {
          return;
        } else if (isButtonEvent(key, GameButton.A)) {
          const selectedItem = items[selection];
          if (selectedItem.identifier === "CANCEL") {
            return;
          }
          if (selectedItem.price <= 0) {
            await this.showMessageAsync(ui, "That item isn't for sale right now.");
            break;
          }
          const quantity = await this.promptQuantityAsync(ui, selectedItem, "buy");
          if (quantity === null) {
            break;
          }
          const purchase = this.shop.buyItem(selectedItem, quantity);
          if (!purchase.success) {
            await this.showMessageAsync(ui, purchase.message);
            break;
          }
          await this.showMessageAsync(ui, MART_TEXT.thanks);
          break;
        }
      }
      await nextFrame();
    }
  }

  private runSellFlow(ui: MartUI): void {
    let sellableItems = this.listSellableInventory();
    if (!sellableItems.length) {
      this.showMessage(ui, "You don't have anything to sell.");
      return;
    }
    let selection = 0;
    let scroll = 0;
    while (true) {
      sellableItems = sellableItems.filter((item) => (item.quantity ?? 0) > 0);
      if (!sellableItems.length) {
        this.showMessage(ui, "That's everything. Thanks!");
        return;
      }
      selection = Math.min(selection, sellableItems.length - 1);
      scroll = Math.min(scroll, Math.max(0, sellableItems.length - MART_MENU_PAGE_SIZE));
      this.drawBase();
      this.drawMoney(ui);
      this.drawItemList(ui, sellableItems, selection, scroll, "sell");
      this.renderSellMenuSnapshot(ui, sellableItems, selection, scroll);
      this.present(ui);
      for (const event of this.iterEvents()) {
        if (event.type === "quit") {
          throw new Error("Quit requested while mart interface active.");
        }
        if (!isKeyDownEvent(event)) {
          continue;
        }
        const key = event.code ?? event.key ?? "";
        const direction = mapKeyToDirection(key ?? null);
        if (direction === "up") {
          selection = Math.max(0, selection - 1);
          if (selection < scroll) {
            scroll = selection;
          }
        } else if (direction === "down") {
          selection = Math.min(sellableItems.length - 1, selection + 1);
          if (selection >= scroll + MART_MENU_PAGE_SIZE) {
            scroll = selection - MART_MENU_PAGE_SIZE + 1;
          }
        } else if (isButtonEvent(key, GameButton.B)) {
          return;
        } else if (isButtonEvent(key, GameButton.A)) {
          const selectedItem = sellableItems[selection];
          const quantity = this.promptQuantity(ui, selectedItem, "sell");
          if (quantity === null) {
            break;
          }
          const sale = this.shop.sellItem(selectedItem, quantity);
          if (!sale.success) {
            this.showMessage(ui, sale.message);
            break;
          }
          const remaining = (selectedItem.quantity ?? 0) - quantity;
          selectedItem.quantity = Math.max(0, remaining);
          this.showMessage(ui, `Sold for ${sale.message}!`);
          break;
        }
      }
    }
  }

  private async runSellFlowAsync(ui: MartUI): Promise<void> {
    let sellableItems = this.listSellableInventory();
    if (!sellableItems.length) {
      await this.showMessageAsync(ui, "You don't have anything to sell.");
      return;
    }
    let selection = 0;
    let scroll = 0;
    while (true) {
      sellableItems = sellableItems.filter((item) => (item.quantity ?? 0) > 0);
      if (!sellableItems.length) {
        await this.showMessageAsync(ui, "That's everything. Thanks!");
        return;
      }
      selection = Math.min(selection, sellableItems.length - 1);
      scroll = Math.min(scroll, Math.max(0, sellableItems.length - MART_MENU_PAGE_SIZE));
      this.drawBase();
      this.drawMoney(ui);
      this.drawItemList(ui, sellableItems, selection, scroll, "sell");
      this.renderSellMenuSnapshot(ui, sellableItems, selection, scroll);
      this.present(ui);
      for (const event of this.iterEvents()) {
        if (event.type === "quit") {
          throw new Error("Quit requested while mart interface active.");
        }
        if (!isKeyDownEvent(event)) {
          continue;
        }
        const key = event.code ?? event.key ?? "";
        const direction = mapKeyToDirection(key ?? null);
        if (direction === "up") {
          selection = Math.max(0, selection - 1);
          if (selection < scroll) {
            scroll = selection;
          }
        } else if (direction === "down") {
          selection = Math.min(sellableItems.length - 1, selection + 1);
          if (selection >= scroll + MART_MENU_PAGE_SIZE) {
            scroll = selection - MART_MENU_PAGE_SIZE + 1;
          }
        } else if (isButtonEvent(key, GameButton.B)) {
          return;
        } else if (isButtonEvent(key, GameButton.A)) {
          const selectedItem = sellableItems[selection];
          const quantity = await this.promptQuantityAsync(ui, selectedItem, "sell");
          if (quantity === null) {
            break;
          }
          const sale = this.shop.sellItem(selectedItem, quantity);
          if (!sale.success) {
            await this.showMessageAsync(ui, sale.message);
            break;
          }
          const remaining = (selectedItem.quantity ?? 0) - quantity;
          selectedItem.quantity = Math.max(0, remaining);
          await this.showMessageAsync(ui, `Sold for ${sale.message}!`);
          break;
        }
      }
      await nextFrame();
    }
  }

  private promptQuantity(ui: MartUI, item: MartItem, mode: "buy" | "sell"): number | null {
    let maxQuantity = 1;
    if (mode === "buy") {
      const maxCapacity = this.shop.maxBuyQuantity(item);
      if (item.price <= 0) {
        this.showMessage(ui, "That item can't be sold right now.");
        return null;
      }
      if (maxCapacity <= 0) {
        if (Math.floor(this.gameState.sram.money / item.price) <= 0) {
          this.showMessage(ui, MART_TEXT.noMoney);
        } else {
          this.showMessage(ui, MART_TEXT.bagFull);
        }
        return null;
      }
      maxQuantity = maxCapacity;
    } else {
      const owned = item.quantity ?? 0;
      if (owned <= 0) {
        this.showMessage(ui, "You don't have any left.");
        return null;
      }
      maxQuantity = owned;
    }
    let quantity = 1;
    while (true) {
      const totalCost = mode === "buy" ? item.price * quantity : sellPrice(item) * quantity;
      this.drawBase();
      this.drawMoney(ui);
      this.drawQuantityPrompt(ui, item, quantity, maxQuantity, totalCost, mode);
      this.renderQuantitySnapshot(ui, item, quantity, maxQuantity, totalCost, mode);
      this.present(ui);
      for (const event of this.iterEvents()) {
        if (event.type === "quit") {
          throw new Error("Quit requested while mart interface active.");
        }
        if (!isKeyDownEvent(event)) {
          continue;
        }
        const key = event.code ?? event.key ?? "";
        const direction = mapKeyToDirection(key ?? null);
        if (direction === "up") {
          quantity = Math.min(maxQuantity, quantity + 1);
        } else if (direction === "down") {
          quantity = Math.max(1, quantity - 1);
        } else if (direction === "right") {
          quantity = Math.min(maxQuantity, quantity + 10);
        } else if (direction === "left") {
          quantity = Math.max(1, quantity - 10);
        } else if (isButtonEvent(key, GameButton.B)) {
          return null;
        } else if (isButtonEvent(key, GameButton.A)) {
          return quantity;
        }
      }
    }
  }

  private async promptQuantityAsync(
    ui: MartUI,
    item: MartItem,
    mode: "buy" | "sell"
  ): Promise<number | null> {
    let maxQuantity = 1;
    if (mode === "buy") {
      const maxCapacity = this.shop.maxBuyQuantity(item);
      if (item.price <= 0) {
        await this.showMessageAsync(ui, "That item can't be sold right now.");
        return null;
      }
      if (maxCapacity <= 0) {
        if (Math.floor(this.gameState.sram.money / item.price) <= 0) {
          await this.showMessageAsync(ui, MART_TEXT.noMoney);
        } else {
          await this.showMessageAsync(ui, MART_TEXT.bagFull);
        }
        return null;
      }
      maxQuantity = maxCapacity;
    } else {
      const owned = item.quantity ?? 0;
      if (owned <= 0) {
        await this.showMessageAsync(ui, "You don't have any left.");
        return null;
      }
      maxQuantity = owned;
    }
    let quantity = 1;
    while (true) {
      const totalCost = mode === "buy" ? item.price * quantity : sellPrice(item) * quantity;
      this.drawBase();
      this.drawMoney(ui);
      this.drawQuantityPrompt(ui, item, quantity, maxQuantity, totalCost, mode);
      this.renderQuantitySnapshot(ui, item, quantity, maxQuantity, totalCost, mode);
      this.present(ui);
      for (const event of this.iterEvents()) {
        if (event.type === "quit") {
          throw new Error("Quit requested while mart interface active.");
        }
        if (!isKeyDownEvent(event)) {
          continue;
        }
        const key = event.code ?? event.key ?? "";
        const direction = mapKeyToDirection(key ?? null);
        if (direction === "up") {
          quantity = Math.min(maxQuantity, quantity + 1);
        } else if (direction === "down") {
          quantity = Math.max(1, quantity - 1);
        } else if (direction === "right") {
          quantity = Math.min(maxQuantity, quantity + 10);
        } else if (direction === "left") {
          quantity = Math.max(1, quantity - 10);
        } else if (isButtonEvent(key, GameButton.B)) {
          return null;
        } else if (isButtonEvent(key, GameButton.A)) {
          return quantity;
        }
      }
      await nextFrame();
    }
  }

  private drawBase(): void {
    if (typeof this.overworld.draw === "function") {
      this.overworld.draw();
    }
  }

  private drawTopMenu(ui: MartUI, options: string[], selection: number): void {
    const [x, y, width, heightTiles] = MART_LAYOUT.topMenu.toPixels(this.tileSize);
    const height = Math.max(heightTiles, 2 + options.length * 2);
    ui.drawWindow(ui.screen!, x, y, width, height, { fill: [255, 255, 255] });
    options.forEach((option, index) => {
      const cursor = index === selection ? "▶" : " ";
      const textX = x + this.tileSize;
      const textY = y + this.tileSize + index * this.tileSize * 2;
      renderFontText(ui.font, `${cursor}${option}`, textX, textY, ui.screen!, {
        textWidth: (width - 2) * this.tileSize,
      });
    });
  }

  private drawItemList(
    ui: MartUI,
    items: MartItem[],
    selection: number,
    scroll: number,
    mode: "buy" | "sell"
  ): void {
    const maxVisible = MART_MENU_PAGE_SIZE;
    const visibleItems = items.slice(scroll, scroll + maxVisible);
    const [x, y, width, height] = MART_LAYOUT.itemList.toPixels(this.tileSize);
    ui.drawWindow(ui.screen!, x, y, width, height, { fill: [255, 255, 255] });
    let currentItem: MartItem | null = null;
    visibleItems.forEach((item, index) => {
      const absoluteIndex = scroll + index;
      const cursor = absoluteIndex === selection ? "▶" : " ";
      const textX = x + this.tileSize;
      const textY = y + this.tileSize + index * this.tileSize * 2;
      if (mode === "buy") {
        const line = `${cursor}${item.displayName}`;
        renderFontText(ui.font, line, textX, textY, ui.screen!, { textWidth: (width - 2) * this.tileSize });
        if (item.identifier !== "CANCEL") {
          const priceX = textX + BUY_PRICE_OFFSET_TILES * this.tileSize;
          const priceY = textY + this.tileSize;
          renderFontText(ui.font, formatPrice(item.price), priceX, priceY, ui.screen!, {
            textWidth: (width - 2 - BUY_PRICE_OFFSET_TILES) * this.tileSize,
          });
        }
      } else {
        const qty = item.quantity ?? 0;
        const qtyText = item.identifier === "CANCEL" ? "" : `×${String(qty).padStart(2, "0")}`;
        const line = qtyText ? `${cursor}${item.displayName} ${qtyText}` : `${cursor}${item.displayName}`;
        renderFontText(ui.font, line, textX, textY, ui.screen!, { textWidth: (width - 2) * this.tileSize });
      }
      if (absoluteIndex === selection) {
        currentItem = item;
      }
    });
    if (!currentItem && selection >= 0 && selection < items.length) {
      currentItem = items[selection];
    }
    this.drawItemDescription(ui, currentItem);
  }

  private drawItemDescription(ui: MartUI, item: MartItem | null): void {
    const [x, y, width, height] = MART_LAYOUT.descriptionBox.toPixels(this.tileSize);
    ui.drawWindow(ui.screen!, x, y, width, height, { fill: [255, 255, 255] });
    if (!item || item.identifier === "CANCEL") {
      return;
    }
    const definition = this.itemSystem.getItemDefinition(item.identifier);
    const description = definition?.description ?? "NO DESCRIPTION AVAILABLE.";
    const innerWidth = width - 2;
    const innerHeight = height - 2;
    const lines = wrapText(description, innerWidth, innerHeight);
    lines.slice(0, innerHeight).forEach((line, index) => {
      renderFontText(ui.font, line, x + this.tileSize, y + this.tileSize + index * this.tileSize, ui.screen!, {
        textWidth: innerWidth * this.tileSize,
      });
    });
  }

  private drawQuantityPrompt(
    ui: MartUI,
    _item: MartItem,
    quantity: number,
    _maxQuantity: number,
    totalCost: number,
    _mode: "buy" | "sell"
  ): void {
    const [x, y, width, height] = MART_LAYOUT.quantityPrompt.toPixels(this.tileSize);
    ui.drawWindow(ui.screen!, x, y, width, height, { fill: [255, 255, 255] });
    const interiorX = x + this.tileSize;
    const interiorY = y + this.tileSize;
    const quantityText = `×${String(quantity).padStart(2, "0")}`;
    renderFontText(ui.font, quantityText, interiorX, interiorY, ui.screen!, {
      textWidth: (width - 2) * this.tileSize,
    });
    const priceText = formatPrice(totalCost);
    const priceX = interiorX + this.tileSize * 4;
    renderFontText(ui.font, priceText, priceX, interiorY, ui.screen!, {
      textWidth: (width - 5) * this.tileSize,
    });
  }

  private drawMoney(ui: MartUI): void {
    const money = this.gameState.sram.money ?? 0;
    const text = formatPrice(money);
    const [x, y, widthTiles, heightTiles] = MART_LAYOUT.moneyWindow.toPixels(this.tileSize);
    ui.drawWindow(ui.screen!, x, y, widthTiles, heightTiles, { fill: [255, 255, 255] });
    renderFontText(ui.font, text, x + this.tileSize, y + this.tileSize, ui.screen!, {
      textWidth: (widthTiles - 2) * this.tileSize,
    });
  }

  private listSellableInventory(): MartItem[] {
    const sellable: MartItem[] = [];
    const inventory = this.itemSystem.listItems();
    for (const [identifier, quantity] of Object.entries(inventory)) {
      if (quantity <= 0) {
        continue;
      }
      const definition = this.itemSystem.getItemDefinition(identifier);
      const price = definition?.price ?? 0;
      let pocket = definition?.pocket ?? ItemPocket.ITEM;
      if (typeof pocket === "string") {
        pocket = ItemPocket[pocket as keyof typeof ItemPocket] ?? ItemPocket.ITEM;
      }
      if (pocket === ItemPocket.KEY_ITEM || price <= 0) {
        continue;
      }
      const displayName = this.itemSystem.getDisplayName(identifier);
      sellable.push({ identifier, displayName, price, quantity });
    }
    sellable.sort((a, b) => a.displayName.localeCompare(b.displayName));
    return sellable;
  }

  private renderTopMenuSnapshot(ui: MartUI, options: string[], selection: number): void {
    renderTextSnapshot(ui, {
      viewportLines: this.menuViewportLines("MART"),
      infoLines: MART_MENU_CONTROLS,
      viewportTitle: MART_VIEWPORT_TITLE,
      infoTitle: MART_INFO_TITLE,
      menuLines: options.map((option, index) => cursorLine(option, index === selection)),
    });
  }

  private renderBuyMenuSnapshot(ui: MartUI, items: MartItem[], selection: number, scroll: number): void {
    renderTextSnapshot(ui, {
      viewportLines: this.menuViewportLines("BUY"),
      infoLines: MART_MENU_CONTROLS,
      viewportTitle: MART_VIEWPORT_TITLE,
      infoTitle: MART_INFO_TITLE,
      menuLines: this.buildBuyMenuLines(items, selection, scroll),
    });
  }

  private renderSellMenuSnapshot(ui: MartUI, items: MartItem[], selection: number, scroll: number): void {
    renderTextSnapshot(ui, {
      viewportLines: this.menuViewportLines("SELL"),
      infoLines: MART_MENU_CONTROLS,
      viewportTitle: MART_VIEWPORT_TITLE,
      infoTitle: MART_INFO_TITLE,
      menuLines: this.buildSellMenuLines(items, selection, scroll),
    });
  }

  private renderQuantitySnapshot(
    ui: MartUI,
    item: MartItem,
    quantity: number,
    _maxQuantity: number,
    totalCost: number,
    mode: "buy" | "sell"
  ): void {
    const label = mode === "buy" ? "BUY" : "SELL";
    const promptLines = [
      `${item.displayName} x${String(quantity).padStart(2, "0")}`,
      `TOTAL: ${formatPrice(totalCost)}`,
    ];
    renderTextSnapshot(ui, {
      viewportLines: this.menuViewportLines(label),
      infoLines: MART_PROMPT_CONTROLS,
      viewportTitle: MART_VIEWPORT_TITLE,
      infoTitle: MART_INFO_TITLE,
      promptLines,
    });
  }

  private renderMessageSnapshot(ui: MartUI, text: string): void {
    const lines = wrapText(text, SCREEN_TILE_WIDTH - 2, TEXTBOX_HEIGHT_TILES - 2);
    renderTextSnapshot(ui, {
      viewportLines: this.menuViewportLines("MART"),
      infoLines: MART_DIALOGUE_CONTROLS,
      viewportTitle: MART_VIEWPORT_TITLE,
      infoTitle: MART_INFO_TITLE,
      dialogueLines: lines,
    });
  }

  private menuViewportLines(mode: string): string[] {
    const money = this.gameState.sram.money ?? 0;
    const header = mode === "MART" ? "MART" : `MART - ${mode}`;
    return [header, `MONEY: ${formatPrice(money)}`];
  }

  private buildBuyMenuLines(items: MartItem[], selection: number, scroll: number): string[] {
    const lines: string[] = [];
    const visible = items.slice(scroll, scroll + MART_MENU_PAGE_SIZE);
    visible.forEach((item, index) => {
      const absoluteIndex = scroll + index;
      lines.push(cursorLine(item.displayName, absoluteIndex === selection));
      if (item.identifier !== "CANCEL") {
        lines.push(`  ${formatPrice(item.price)}`);
      } else {
        lines.push("");
      }
    });
    return lines;
  }

  private buildSellMenuLines(items: MartItem[], selection: number, scroll: number): string[] {
    const visible = items.slice(scroll, scroll + MART_MENU_PAGE_SIZE);
    return visible.map((item, index) => {
      const absoluteIndex = scroll + index;
      const qty = item.quantity ?? 0;
      const qtyText = item.identifier === "CANCEL" ? "" : `×${String(qty).padStart(2, "0")}`;
      const label = qtyText ? `${item.displayName} ${qtyText}` : item.displayName;
      return cursorLine(label, absoluteIndex === selection);
    });
  }

  private present(ui: MartUI): void {
    ui.update();
  }

  private showMessage(ui: MartUI, text: string): void {
    let waiting = true;
    while (waiting) {
      this.drawBase();
      const x = 0;
      const y = TEXTBOX_Y_TILES * this.tileSize;
      const width = SCREEN_TILE_WIDTH;
      const height = TEXTBOX_HEIGHT_TILES;
      ui.drawWindow(ui.screen!, x, y, width, height, { fill: [255, 255, 255] });
      renderFontText(ui.font, text, x + this.tileSize, y + this.tileSize, ui.screen!, {
        textWidth: (width - 2) * this.tileSize,
        maxLines: height - 2,
      });
      this.renderMessageSnapshot(ui, text);
      this.present(ui);
      for (const event of this.iterEvents()) {
        if (event.type === "quit") {
          throw new Error("Quit requested while mart interface active.");
        }
        if (isKeyDownEvent(event)) {
          const key = event.code ?? event.key ?? "";
          if (isButtonEvent(key, GameButton.A) || isButtonEvent(key, GameButton.B)) {
            waiting = false;
            break;
          }
        }
      }
    }
  }

  private async showMessageAsync(ui: MartUI, text: string): Promise<void> {
    let waiting = true;
    while (waiting) {
      this.drawBase();
      const x = 0;
      const y = TEXTBOX_Y_TILES * this.tileSize;
      const width = SCREEN_TILE_WIDTH;
      const height = TEXTBOX_HEIGHT_TILES;
      ui.drawWindow(ui.screen!, x, y, width, height, { fill: [255, 255, 255] });
      renderFontText(ui.font, text, x + this.tileSize, y + this.tileSize, ui.screen!, {
        textWidth: (width - 2) * this.tileSize,
        maxLines: height - 2,
      });
      this.renderMessageSnapshot(ui, text);
      this.present(ui);
      for (const event of this.iterEvents()) {
        if (event.type === "quit") {
          throw new Error("Quit requested while mart interface active.");
        }
        if (isKeyDownEvent(event)) {
          const key = event.code ?? event.key ?? "";
          if (isButtonEvent(key, GameButton.A) || isButtonEvent(key, GameButton.B)) {
            waiting = false;
            break;
          }
        }
      }
      if (waiting) {
        await nextFrame();
      }
    }
  }

  private suppressScriptDialogue(): () => void {
    const dialogue = this.overworld.dialogue;
    if (!dialogue) {
      return () => undefined;
    }
    if (dialogue._suspended || typeof dialogue.suspend !== "function" || typeof dialogue.resume !== "function") {
      return () => undefined;
    }
    dialogue.suspend();
    return () => {
      if (typeof dialogue.resume === "function") {
        dialogue.resume();
      }
    };
  }
}

const wrapText = (text: string, widthTiles: number, maxLines: number): string[] => {
  if (widthTiles <= 0 || maxLines <= 0) {
    return [];
  }
  const normalized = text.replace(/\s+/g, " ").trim();
  if (!normalized) {
    return [];
  }
  const words = normalized.split(" ");
  const lines: string[] = [];
  let current = "";
  for (const word of words) {
    const candidate = current ? `${current} ${word}` : word;
    if (candidate.length <= widthTiles) {
      current = candidate;
      continue;
    }
    if (current) {
      lines.push(current);
      current = word;
    } else {
      lines.push(word.slice(0, widthTiles));
      current = word.slice(widthTiles);
    }
  }
  if (current) {
    lines.push(current);
  }
  if (lines.length <= maxLines) {
    return lines;
  }
  const trimmed = lines.slice(0, maxLines);
  trimmed[maxLines - 1] = trimmed[maxLines - 1].slice(0, Math.max(0, widthTiles - 3)).trimEnd() + "...";
  return trimmed;
};

const sellPrice = (item: MartItem): number => {
  return Math.max(0, Math.floor(item.price / 2));
};

const cursorLine = (label: string, active: boolean): string => {
  const prefix = active ? "▶" : "  ";
  return `${prefix} ${label}`;
};
