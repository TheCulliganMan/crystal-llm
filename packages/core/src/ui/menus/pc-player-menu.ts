// ASM mapping: pokecrystal_disassembly/engine/events/pokecenter_pc.asm::_PlayersPC
// and pokecrystal_disassembly/engine/menus/player_pc.asm (Player's PC item storage flow).
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { ItemSystem, type ItemSystemDataLoader } from "@pokecrystal/core/engine/systems/items";
import { GameState } from "@pokecrystal/core/core/state";
import { MAX_PC_ITEMS, MAX_PC_ITEM_QUANTITY } from "@pokecrystal/core/core/constants";
import {
  KeyEvent,
  isCancelEvent,
  isConfirmEvent,
  isKeyDownEvent,
} from "@pokecrystal/core/input/buttons";
import { Surface } from "@pokecrystal/core/ui/surface";
import { TilemapSurface, SPACE_TILE } from "@pokecrystal/core/ui/tilemap-surface";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { blitPcTilemap, createPcTilemap, PC_TEXT_PALETTE, BitmapFontLike } from "./pc-wallpaper";
import { GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import type { RenderTextOptions } from "@pokecrystal/core/ui/font-renderer";
import type { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";

const PLAYER_PC_MENU_REGION = { x: 0, y: 0, width: 16, height: 13 } as const;
const PLAYER_PC_ITEMS_TEXTBOX_REGION = { x: 0, y: 0, width: 20, height: 12 } as const;
const PLAYER_PC_ITEMS_MENU = { x: 4, firstRowY: 2, rows: 4, columns: 8, rowHeight: 2 } as const;
const PLAYER_PC_DESCRIPTION_REGION = { x: 0, y: 12, width: 20, height: 6 } as const;

type PlayerPCMenuAction = "withdraw" | "deposit" | "toss";
type PlayerPCMenuSpecialAction = "mail_box";

type PCMenuActionStatus =
  | "missing_item"
  | "invalid"
  | "empty"
  | "bag_error"
  | "pc_full"
  | "bag_full"
  | "ok"
  | "unknown";

type PCMenuPendingAction = {
  type: PlayerPCMenuAction | null;
  name: string;
  display: string;
  max: number;
};

type PCMenuScriptEntry = {
  action?: string | null;
  item?: string | null;
  quantity?: number | string;
};

export interface PlayerPCMenuActionResult {
  action: string | null;
  status: PCMenuActionStatus;
  item?: string | null;
  display?: string;
  quantity?: number;
  mailbox?: Record<string, unknown>;
}

export interface PlayerPCUI {
  screen: Surface | null;
  drawWindow: (
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    options?: { fill?: [number, number, number] | null }
  ) => void;
  font: BitmapFontLike & {
    renderText: (
      text: string,
      x: number,
      y: number,
      surface: Surface,
      options?: RenderTextOptions | boolean
    ) => void;
  };
  update: () => void;
  eventQueue?: GameEngineEventQueue;
  pollEvents?: () => KeyEvent[];
  renderSnapshot?: ScreenUI["renderSnapshot"];
}

export class PlayerPCMenu {
  static readonly POKECENTER_ACTIONS = [
    "WITHDRAW ITEM",
    "DEPOSIT ITEM",
    "TOSS ITEM",
    "MAIL BOX",
    "LOG OFF",
  ] as const;
  static readonly PLAYERS_HOUSE_ACTIONS = [
    "WITHDRAW ITEM",
    "DEPOSIT ITEM",
    "TOSS ITEM",
    "MAIL BOX",
    "DECORATION",
    "TURN OFF",
  ] as const;

  private mode: "menu" | "list" | "quantity" = "menu";
  private menuIndex = 0;
  private listIndex = 0;
  private listOffset = 0;
  private currentAction: PlayerPCMenuAction | null = null;
  private quantity = 1;
  private pendingAction: PCMenuPendingAction | null = null;

  private readonly itemSystem: ItemSystem;
  private readonly menuActions: readonly string[];

  constructor(
    private readonly ui: PlayerPCUI,
    private readonly gameState: GameState,
    private readonly dataLoader?: ItemSystemDataLoader,
    private readonly audioEngine?: AudioEngine | null,
    options?: { menuActions?: readonly string[] }
  ) {
    this.itemSystem = new ItemSystem(gameState, dataLoader);
    this.menuActions = options?.menuActions ?? PlayerPCMenu.POKECENTER_ACTIONS;
  }

  jumpToAction(action: string, { openList = false }: { openList?: boolean } = {}): void {
    const normalized = action.trim().toUpperCase();
    const selectionIndex = this.menuActions.findIndex(
      (candidate) => candidate === normalized,
    );
    if (selectionIndex === -1) {
      return;
    }
    this.menuIndex = selectionIndex;
    const selection = this.menuActions[selectionIndex];
    this.currentAction = PlayerPCMenu.actionFromLabel(selection);
    if (openList) {
      if (!this.currentAction) {
        this.mode = "menu";
        return;
      }
      this.mode = "list";
      this.listIndex = 0;
      this.listOffset = 0;
      this.ensureListVisible(this.currentItems().length);
    } else {
      this.mode = "menu";
    }
  }

  handleInput(event: KeyEvent): PlayerPCMenuActionResult | "cancel" | null {
    if (!isKeyDownEvent(event) || !this.ui.screen) {
      return null;
    }
    const keyName = typeof event.key === "string" ? event.key : typeof event.code === "string" ? event.code : "";
    if (this.mode === "menu") {
      return this.handleMenuInput(event, keyName);
    }
    if (this.mode === "list") {
      return this.handleListInput(event, keyName);
    }
    if (this.mode === "quantity") {
      return this.handleQuantityInput(event, keyName);
    }
    return null;
  }

  private handleMenuInput(event: KeyEvent, keyName: string): PlayerPCMenuActionResult | "cancel" | null {
    if (keyName === "ArrowUp" || keyName === "ArrowLeft") {
      this.menuIndex = (this.menuIndex - 1 + this.menuActions.length) % this.menuActions.length;
      this.playCursor();
      return null;
    }
    if (keyName === "ArrowDown" || keyName === "ArrowRight") {
      this.menuIndex = (this.menuIndex + 1) % this.menuActions.length;
      this.playCursor();
      return null;
    }
    if (isCancelEvent(event)) {
      this.playConfirm();
      return "cancel";
    }
    if (!isConfirmEvent(event)) {
      return null;
    }
    const selection = this.menuActions[this.menuIndex];
    if (selection === "LOG OFF" || selection === "TURN OFF") {
      return "cancel";
    }
    if (selection === "MAIL BOX") {
      this.playConfirm();
      return { action: "mail_box" satisfies PlayerPCMenuSpecialAction, status: "ok" };
    }
    if (selection === "DECORATION") {
      this.playConfirm();
      return { action: "decoration", status: "unknown" };
    }
    this.currentAction = PlayerPCMenu.actionFromLabel(selection);
    this.listIndex = 0;
    this.listOffset = 0;
    this.mode = "list";
    this.playConfirm();
    return null;
  }

  private static actionFromLabel(label: string): PlayerPCMenuAction | null {
    const action = label.split(" ")[0].toLowerCase();
    if (action === "withdraw" || action === "deposit" || action === "toss") {
      return action;
    }
    return null;
  }

  private handleListInput(event: KeyEvent, keyName: string): PlayerPCMenuActionResult | "cancel" | null {
    const items = this.currentItems();
    if (!items.length) {
      if (isConfirmEvent(event)) {
        return {
          action: this.currentAction,
          status: "empty",
          display: this.currentAction ? this.currentAction.toUpperCase() : "ITEM",
        };
      }
      if (isCancelEvent(event)) {
        this.mode = "menu";
        this.playConfirm();
      }
      return null;
    }
    if (keyName === "ArrowUp" || keyName === "ArrowLeft") {
      this.listIndex = (this.listIndex - 1 + items.length) % items.length;
      this.ensureListVisible(items.length);
      this.playCursor();
      return null;
    }
    if (keyName === "ArrowDown" || keyName === "ArrowRight") {
      this.listIndex = (this.listIndex + 1) % items.length;
      this.ensureListVisible(items.length);
      this.playCursor();
      return null;
    }
    if (isCancelEvent(event)) {
      this.mode = "menu";
      this.playConfirm();
      return null;
    }
    if (!isConfirmEvent(event)) {
      return null;
    }
    const entry = items[this.listIndex];
    this.quantity = 1;
    this.pendingAction = {
      type: this.currentAction,
      name: entry.name,
      display: entry.display,
      max: entry.quantity,
    };
    this.mode = "quantity";
    return null;
  }

  private handleQuantityInput(event: KeyEvent, keyName: string): PlayerPCMenuActionResult | "cancel" | null {
    if (!this.pendingAction) {
      this.mode = "list";
      return null;
    }
    const maxQty = this.pendingAction.max;
    if (keyName === "ArrowUp" || keyName === "ArrowRight") {
      this.quantity = Math.min(maxQty, this.quantity + 1);
      this.playCursor();
      return null;
    }
    if (keyName === "ArrowDown" || keyName === "ArrowLeft") {
      this.quantity = Math.max(1, this.quantity - 1);
      this.playCursor();
      return null;
    }
    if (isCancelEvent(event)) {
      this.mode = "list";
      this.playConfirm();
      return null;
    }
    if (!isConfirmEvent(event)) {
      return null;
    }
    const pending = this.pendingAction;
    const result = this.processAction(pending.type, pending.name, this.quantity);
    this.pendingAction = null;
    this.mode = "list";
    this.playConfirm();
    this.ensureListVisible(this.currentItems().length);
    return result;
  }

  draw(): void {
    if (!this.ui.screen) {
      return;
    }
    this.ui.screen.fill([255, 255, 255, 255]);
    const tilemap = this.renderTilemap();
    this.renderTextSnapshot();
    blitPcTilemap(this.ui.screen, this.ui.font, tilemap);
  }

  renderTilemap(): TilemapSurface {
    const { tilemap } = createPcTilemap();
    if (this.mode === "menu") {
      this.drawActions(tilemap);
      return tilemap;
    }
    this.drawItemStorage(tilemap);
    return tilemap;
  }

  private renderTextSnapshot(): void {
    if (!this.ui.renderSnapshot) {
      return;
    }
    const viewportLines = ["PLAYER'S PC", this.statusText()];
    const items = this.currentItems();
    if (this.mode === "list" || this.mode === "quantity") {
      const visibleCount = PLAYER_PC_ITEMS_MENU.rows;
      const visible = items.slice(this.listOffset, this.listOffset + visibleCount);
      if (visible.length) {
        if (this.listOffset > 0) {
          viewportLines.push("▲ more above");
        }
        visible.forEach((entry, offset) => {
          const index = this.listOffset + offset;
          const cursor = index === this.listIndex && this.mode === "list" ? "▶" : " ";
          viewportLines.push(`${cursor} ${entry.display} x${String(entry.quantity).padStart(2, "0")}`);
        });
        if (this.listOffset + visible.length < items.length) {
          viewportLines.push("▼ more below");
        }
      } else {
        viewportLines.push("  NO ITEMS");
      }
    }
    const menuLines = this.mode === "menu"
      ? this.menuActions.map((label, index) => `${index === this.menuIndex ? "▶" : " "} ${label}`)
      : null;
    const promptLines = this.mode === "quantity" && this.pendingAction
      ? [`${this.pendingAction.display} x${this.quantity}`]
      : null;
    this.ui.renderSnapshot(
      viewportLines,
      ["D-Pad=Move A=Select B=Back"],
      "Player's PC",
      "Legend",
      menuLines,
      promptLines,
      null,
    );
  }

  private drawItemStorage(tilemap: TilemapSurface): void {
    tilemap.drawWindow(
      PLAYER_PC_ITEMS_TEXTBOX_REGION.x,
      PLAYER_PC_ITEMS_TEXTBOX_REGION.y,
      PLAYER_PC_ITEMS_TEXTBOX_REGION.width,
      PLAYER_PC_ITEMS_TEXTBOX_REGION.height,
      {
        attr: PC_TEXT_PALETTE,
        fillTile: SPACE_TILE,
      },
    );
    this.drawItemStorageList(tilemap);
    this.drawItemDescription(tilemap);
  }

  private drawItemStorageList(tilemap: TilemapSurface): void {
    const items = this.currentItems();
    const maxRows = PLAYER_PC_ITEMS_MENU.rows;
    const nameX = PLAYER_PC_ITEMS_MENU.x + 1;
    const cursorX = PLAYER_PC_ITEMS_MENU.x;
    const quantityX = nameX + PLAYER_PC_ITEMS_MENU.columns;
    for (let row = 0; row < maxRows; row++) {
      const idx = this.listOffset + row;
      if (idx >= items.length) {
        break;
      }
      const entry = items[idx];
      const y = PLAYER_PC_ITEMS_MENU.firstRowY + row * PLAYER_PC_ITEMS_MENU.rowHeight;
      tilemap.writeText(cursorX, y, idx === this.listIndex && this.mode === "list" ? "▶" : " ", {
        maxLength: 1,
        pad: true,
      });
      tilemap.writeText(nameX, y, entry.display, {
        maxLength: PLAYER_PC_ITEMS_MENU.columns,
        pad: true,
      });
      tilemap.writeText(quantityX, y, `×${String(entry.quantity).padStart(2, "0")}`, {
        maxLength: 3,
        pad: true,
      });
    }
    if (this.mode === "quantity" && this.pendingAction) {
      const y = PLAYER_PC_ITEMS_MENU.firstRowY + (maxRows - 1) * PLAYER_PC_ITEMS_MENU.rowHeight;
      tilemap.writeText(nameX, y, this.pendingAction.display, {
        maxLength: PLAYER_PC_ITEMS_MENU.columns,
        pad: true,
      });
      tilemap.writeText(quantityX, y, `×${String(this.quantity).padStart(2, "0")}`, {
        maxLength: 3,
        pad: true,
      });
    }
  }

  private drawItemDescription(tilemap: TilemapSurface): void {
    tilemap.drawWindow(
      PLAYER_PC_DESCRIPTION_REGION.x,
      PLAYER_PC_DESCRIPTION_REGION.y,
      PLAYER_PC_DESCRIPTION_REGION.width,
      PLAYER_PC_DESCRIPTION_REGION.height,
      {
        attr: PC_TEXT_PALETTE,
        fillTile: SPACE_TILE,
      },
    );
    const selected = this.currentItems()[this.listIndex];
    if (!selected) {
      return;
    }
    const description = this.itemSystem.getItemDefinition(selected.name).description.toUpperCase();
    const words = description.split(/\s+/).filter(Boolean);
    const lines: string[] = [];
    let line = "";
    for (const word of words) {
      const next = line ? `${line} ${word}` : word;
      if (next.length > PLAYER_PC_DESCRIPTION_REGION.width - 2) {
        lines.push(line);
        line = word;
      } else {
        line = next;
      }
      if (lines.length >= 2) {
        break;
      }
    }
    if (line && lines.length < 2) {
      lines.push(line);
    }
    lines.forEach((text, index) => {
      tilemap.writeText(PLAYER_PC_DESCRIPTION_REGION.x + 1, PLAYER_PC_DESCRIPTION_REGION.y + 2 + index, text, {
        maxLength: PLAYER_PC_DESCRIPTION_REGION.width - 2,
        pad: true,
      });
    });
  }

  private drawActions(tilemap: TilemapSurface): void {
    const region = PLAYER_PC_MENU_REGION;
    tilemap.drawWindow(region.x, region.y, region.width, region.height, {
      attr: PC_TEXT_PALETTE,
      fillTile: SPACE_TILE,
    });
    this.menuActions.forEach((label, index) => {
      const cursor = index === this.menuIndex ? "▶" : " ";
      tilemap.writeText(region.x + 1, region.y + 2 + index, `${cursor}${label}`, {
        maxLength: region.width - 3,
        pad: true,
      });
    });
  }

  runInteractive(
    {
      actionHandler,
      drawCallback,
      eventProvider,
    }: {
      actionHandler?: (payload: PlayerPCMenuActionResult) => PlayerPCMenuActionResult | null;
      drawCallback?: () => void;
      eventProvider?: () => KeyEvent[];
    } = {}
  ): PlayerPCMenuActionResult[] {
    if (!this.ui.screen) {
      return [];
    }
    const results: PlayerPCMenuActionResult[] = [];
    let running = true;
    while (running) {
      const events = eventProvider ? eventProvider() : [];
      for (const event of events) {
        if (event.type === "quit") {
          throw new Error("Quit requested while Player PC menu active.");
        }
        const handled = this.handleInput(event);
        if (!handled) {
          continue;
        }
        if (handled === "cancel") {
          running = false;
          break;
        }
        const payload = handled;
        const response = actionHandler ? actionHandler(payload) : payload;
        if (response && typeof response === "object") {
          results.push(response);
        }
      }
      if (drawCallback) {
        drawCallback();
      }
      this.draw();
      this.ui.update();
    }
    return results;
  }

  async runInteractiveAsync(
    {
      actionHandler,
      drawCallback,
      eventProvider,
    }: {
      actionHandler?: (
        payload: PlayerPCMenuActionResult
      ) => PlayerPCMenuActionResult | Promise<PlayerPCMenuActionResult | null> | null;
      drawCallback?: () => void;
      eventProvider?: () => KeyEvent[];
    } = {}
  ): Promise<PlayerPCMenuActionResult[]> {
    if (!this.ui.screen) {
      return [];
    }
    const results: PlayerPCMenuActionResult[] = [];
    let running = true;
    while (running) {
      const events = eventProvider ? eventProvider() : [];
      for (const event of events) {
        if (event.type === "quit") {
          throw new Error("Quit requested while Player PC menu active.");
        }
        const handled = this.handleInput(event);
        if (!handled) {
          continue;
        }
        if (handled === "cancel") {
          running = false;
          break;
        }
        const payload = handled;
        const response = actionHandler ? await actionHandler(payload) : payload;
        if (response && typeof response === "object") {
          results.push(response);
        }
      }
      if (drawCallback) {
        drawCallback();
      }
      this.draw();
      this.ui.update();
      await nextFrame();
    }
    return results;
  }

  scriptedActions(actions?: PCMenuScriptEntry[] | null): PlayerPCMenuActionResult[] {
    if (!actions) {
      return [];
    }
    const results: PlayerPCMenuActionResult[] = [];
    for (const entry of actions) {
      if (!entry || typeof entry !== "object") {
        results.push({ status: "invalid", action: null });
        continue;
      }
      results.push(this.executeScriptedAction(entry));
    }
    return results;
  }

  executeScriptedAction(entry: PCMenuScriptEntry): PlayerPCMenuActionResult {
    const action = String(entry.action ?? "").toLowerCase();
    const item = entry.item;
    const quantity = Number(entry.quantity ?? 1);
    if (!item) {
      return { status: "missing_item", action };
    }
    const canonical = String(item).replace(/ /g, "_").toUpperCase();
    const typedAction = PlayerPCMenu.actionFromLabel(action);
    return this.processAction(typedAction, canonical, quantity);
  }

  private currentItems(): Array<{ name: string; display: string; quantity: number }> {
    const items: Array<{ name: string; display: string; quantity: number }> = [];
    if (this.currentAction === "withdraw") {
      for (const slot of this.gameState.sram.pc_items) {
        if (slot.quantity <= 0) {
          continue;
        }
        const display = this.itemSystem.getDisplayName(slot.item).toUpperCase();
        items.push({ name: slot.item, display, quantity: slot.quantity });
      }
    } else if (this.currentAction === "deposit") {
      const inventory = this.itemSystem.listItems();
      for (const [name, quantity] of Object.entries(inventory)) {
        if (quantity <= 0) {
          continue;
        }
        const display = this.itemSystem.getDisplayName(name).toUpperCase();
        items.push({ name, display, quantity });
      }
    } else if (this.currentAction === "toss") {
      for (const slot of this.gameState.sram.pc_items) {
        if (slot.quantity <= 0) {
          continue;
        }
        const display = this.itemSystem.getDisplayName(slot.item).toUpperCase();
        items.push({ name: slot.item, display, quantity: slot.quantity });
      }
    }
    return items.sort((a, b) => a.display.localeCompare(b.display));
  }

  private statusText(): string {
    if (this.mode === "menu") {
      return "SELECT AN OPTION.";
    }
    if (this.mode === "list") {
      if (!this.currentItems().length) {
        return "NOTHING HERE.";
      }
      return "CHOOSE AN ITEM.";
    }
    if (this.mode === "quantity") {
      return "HOW MANY?";
    }
    return "PLAYER'S PC";
  }

  private ensureListVisible(total: number): void {
    const maxRows = PLAYER_PC_ITEMS_MENU.rows;
    if (total === 0) {
      this.listOffset = 0;
      this.listIndex = 0;
      return;
    }
    if (this.listIndex < 0) {
      this.listIndex = 0;
    }
    if (this.listIndex >= total) {
      this.listIndex = total - 1;
    }
    if (this.listIndex < this.listOffset) {
      this.listOffset = this.listIndex;
    } else if (this.listIndex >= this.listOffset + maxRows) {
      this.listOffset = this.listIndex - maxRows + 1;
    }
  }

  private processAction(action: PlayerPCMenuAction | null, item: string | null, quantity: number): PlayerPCMenuActionResult {
    if (!action || !item) {
      return { action, status: "invalid", item };
    }
    const qty = Math.max(1, quantity);
    const display = this.itemSystem.getDisplayName(item).toUpperCase();
    if (action === "deposit") {
      const available = this.itemSystem.getQuantity(item);
      if (available <= 0) {
        return { action, status: "empty", item, display };
      }
      const actual = Math.min(qty, available);
      if (!this.itemSystem.removeItem(item, actual)) {
        return { action, status: "bag_error", item, display };
      }
      if (!this.addPcItem(item, actual)) {
        this.itemSystem.addItem(item, actual);
        return { action, status: "pc_full", item, display };
      }
      return { action, status: "ok", item, display, quantity: actual };
    }
    if (action === "withdraw") {
      const slot = this.gameState.sram.pc_items.find((entry) => entry.item === item);
      if (!slot || slot.quantity <= 0) {
        return { action, status: "empty", item, display };
      }
      const actual = Math.min(qty, slot.quantity);
      if (!this.itemSystem.addItem(item, actual)) {
        return { action, status: "bag_full", item, display };
      }
      this.removePcItem(item, actual);
      return { action, status: "ok", item, display, quantity: actual };
    }
    if (action === "toss") {
      const slot = this.gameState.sram.pc_items.find((entry) => entry.item === item);
      if (!slot || slot.quantity <= 0) {
        return { action, status: "empty", item, display };
      }
      const actual = Math.min(qty, slot.quantity);
      this.removePcItem(item, actual);
      return { action, status: "ok", item, display, quantity: actual };
    }
    return { action, status: "unknown", item, display };
  }

  private playCursor(): void {}

  private playConfirm(): void {
    if (this.audioEngine) {
      this.audioEngine.playSound("menu_option");
    }
  }

  private addPcItem(item: string, quantity: number): boolean {
    if (quantity <= 0) {
      throw new Error("quantity must be positive");
    }
    const slot = this.findPcSlot(item);
    if (slot) {
      if (slot.quantity + quantity > MAX_PC_ITEM_QUANTITY) {
        return false;
      }
      slot.quantity += quantity;
      return true;
    }
    if (this.gameState.sram.pc_items.length >= MAX_PC_ITEMS) {
      return false;
    }
    this.gameState.sram.pc_items.push({ item, quantity: Math.min(quantity, MAX_PC_ITEM_QUANTITY) });
    return true;
  }

  private removePcItem(item: string, quantity: number): boolean {
    if (quantity <= 0) {
      throw new Error("quantity must be positive");
    }
    const slot = this.findPcSlot(item);
    if (!slot || slot.quantity < quantity) {
      return false;
    }
    slot.quantity -= quantity;
    if (slot.quantity <= 0) {
      const index = this.gameState.sram.pc_items.indexOf(slot);
      if (index >= 0) {
        this.gameState.sram.pc_items.splice(index, 1);
      }
    }
    return true;
  }

  private findPcSlot(item: string): { item: string; quantity: number } | null {
    return this.gameState.sram.pc_items.find((slot) => slot.item === item) ?? null;
  }
}
