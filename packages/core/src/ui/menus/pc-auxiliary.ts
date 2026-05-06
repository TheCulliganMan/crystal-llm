// ASM mapping: pokecrystal_disassembly/engine/events/pokecenter_pc.asm auxiliary PC viewers
// and pokecrystal_disassembly/engine/pokemon/bills_pc_top.asm menu follow-ups.
import { MenuUI } from "./types";
import { AudioEngine } from "../../engine/systems/audio";
import { gameEngine } from "../game-engine";
import { KeyEvent, isCancelEvent, isConfirmEvent, isKeyDownEvent } from "../../input/buttons";
import { TilemapSurface, SPACE_TILE } from "../tilemap-surface";
import { blitPcTilemap, createPcTilemap, PC_TEXT_PALETTE } from "./pc-wallpaper";
import { MailLanguage, MailMessage, preview as mailPreview } from "../../core/mail";
import { ItemEnum } from "../../core/enums";
import { MAILBOX_CAPACITY } from "../../core/constants";
import { nextFrame } from "../async-loop";

const MAILBOX_ACTIONS = ["READ", "TAKE", "GIVE", "DELETE", "CANCEL"] as const;
const MAILBOX_PREVIEW_HEIGHT = 4;
const ITEM_ACTIONS = ["WITHDRAW", "DEPOSIT", "TOSS", "CANCEL"] as const;
const ITEMS_PER_PAGE = 6;

export class ItemPCMenu {
  private scroll = 0;
  private cursor = 0;
  private mode: "list" | "actions" = "list";
  private actionIndex = 0;
  private tilemap: TilemapSurface | null = null;

  constructor(
    private readonly ui: MenuUI,
    private readonly storage: Record<string, number>,
    private readonly audioEngine?: AudioEngine | null,
  ) {}

  private sortedItems(): Array<[string, number]> {
    return Object.entries(this.storage).filter(([, qty]) => qty > 0).sort(([a], [b]) => a.localeCompare(b));
  }

  handleInput(event: KeyEvent): [string, string] | null {
    if (!isKeyDownEvent(event) || !this.ui.screen) {
      return null;
    }
    const key = event.key;
    const items = this.sortedItems();
    if (this.mode === "list") {
      if (key === gameEngine.K_UP && items.length) {
        this.cursor = Math.max(0, this.cursor - 1);
        if (this.cursor < this.scroll) {
          this.scroll = this.cursor;
        }
        this.playCursor();
      } else if (key === gameEngine.K_DOWN && items.length) {
        this.cursor = Math.min(items.length - 1, this.cursor + 1);
        if (this.cursor >= this.scroll + ITEMS_PER_PAGE) {
          this.scroll = this.cursor - ITEMS_PER_PAGE + 1;
        }
        this.playCursor();
      } else if (isConfirmEvent(event)) {
        if (items.length) {
          this.mode = "actions";
          this.actionIndex = 0;
          this.playConfirm();
        }
      } else if (isCancelEvent(event)) {
        this.playConfirm();
        return ["cancel", ""];
      }
    } else {
      if (key === gameEngine.K_UP || key === gameEngine.K_LEFT) {
        this.actionIndex = (this.actionIndex - 1 + ITEM_ACTIONS.length) % ITEM_ACTIONS.length;
        this.playCursor();
      } else if (key === gameEngine.K_DOWN || key === gameEngine.K_RIGHT) {
        this.actionIndex = (this.actionIndex + 1) % ITEM_ACTIONS.length;
        this.playCursor();
      } else if (isCancelEvent(event)) {
        this.mode = "list";
        this.playConfirm();
      } else if (isConfirmEvent(event)) {
        const action = ITEM_ACTIONS[this.actionIndex];
        const item = items[this.cursor]?.[0] ?? "";
        this.playConfirm();
        this.mode = "list";
        if (action === "CANCEL") {
          return null;
        }
        if (action === "TOSS") {
          this.decrement(item);
        }
        return [action.toLowerCase(), item];
      }
    }
    return null;
  }

  draw(): void {
    if (!this.ui.screen) {
      return;
    }
    this.ui.screen.fill([255, 255, 255, 255]);
    const { tilemap } = createPcTilemap();
    this.tilemap = tilemap;
    const listLeft = 1;
    const listTop = 2;
    const listWidth = 18;
    const listHeight = ITEMS_PER_PAGE + 2;
    tilemap.drawWindow(listLeft, listTop, listWidth, listHeight, {
      attr: PC_TEXT_PALETTE,
      fillTile: SPACE_TILE,
    });
    const items = this.sortedItems().slice(this.scroll, this.scroll + ITEMS_PER_PAGE);
    items.forEach(([name, qty], index) => {
      const cursor = this.cursor === this.scroll + index ? "\u25b6" : " ";
      const row = listTop + 1 + index;
      tilemap.writeText(listLeft + 1, row, `${cursor}${name}`, { maxLength: listWidth - 6, pad: true });
      tilemap.writeText(listLeft + 12, row, `\u00d7${String(qty).padStart(2, "0")}`, {
        maxLength: 3,
        pad: true,
      });
    });
    if (this.mode === "actions") {
      const menuLeft = 14;
      const menuTop = 10;
      const menuWidth = 8;
      tilemap.drawWindow(menuLeft, menuTop, menuWidth, ITEM_ACTIONS.length + 2, {
        attr: PC_TEXT_PALETTE,
        fillTile: SPACE_TILE,
      });
      ITEM_ACTIONS.forEach((label, index) => {
        const row = menuTop + 1 + index;
        const cursor = index === this.actionIndex ? "\u25b6" : " ";
        tilemap.writeText(menuLeft + 1, row, `${cursor}${label}`, { maxLength: menuWidth - 2, pad: true });
      });
    }
    blitPcTilemap(this.ui.screen, this.ui.font, tilemap);
    this.renderTextSnapshot(items.map(([name, qty]) => [name, qty] as [string, number]));
  }

  private renderTextSnapshot(items: Array<[string, number]>): void {
    if (!this.ui.renderSnapshot) {
      return;
    }
    const viewportLines = ["ITEM PC"];
    if (items.length) {
      items.forEach(([name, qty], index) => {
        const absoluteIndex = this.scroll + index;
        const cursor = absoluteIndex === this.cursor && this.mode === "list" ? "▶" : " ";
        viewportLines.push(`${cursor} ${name} x${String(qty).padStart(2, "0")}`);
      });
    } else {
      viewportLines.push("  NO ITEMS");
    }
    const menuLines = this.mode === "actions"
      ? ITEM_ACTIONS.map((label, index) => `${index === this.actionIndex ? "▶" : " "} ${label}`)
      : null;
    this.ui.renderSnapshot(
      viewportLines,
      ["D-Pad=Move A=Select B=Back"],
      "Item PC",
      "Legend",
      menuLines,
      null,
      null,
    );
  }

  private decrement(item: string): void {
    const qty = this.storage[item] ?? 0;
    if (qty <= 0) {
      return;
    }
    this.storage[item] = qty - 1;
    if (this.storage[item] <= 0) {
      delete this.storage[item];
      this.cursor = Math.max(0, this.cursor - 1);
      this.scroll = Math.min(this.scroll, this.cursor);
    }
  }

  private playCursor(): void {}

  private playConfirm(): void {
    this.audioEngine?.playSound("menu_option");
  }
}

export class MailboxMenu {
  private cursor = 0;
  private mode: "list" | "actions" = "list";
  private actionIndex = 0;

  constructor(
    private readonly ui: MenuUI | null,
    private readonly messages: MailMessage[],
    private readonly audioEngine?: AudioEngine | null,
  ) {}

  draw(): void {
    if (!this.ui?.screen) {
      return;
    }
    this.ui.screen.fill([255, 255, 255, 255]);
    const { tilemap } = createPcTilemap();
    const height = Math.max(4, Math.min(this.messages.length + 2, 10));
    const listLeft = 1;
    const listTop = 1;
    tilemap.drawWindow(listLeft, listTop, 20, height, {
      attr: PC_TEXT_PALETTE,
      fillTile: SPACE_TILE,
    });
    this.messages.slice(0, height - 2).forEach((mail, index) => {
      const cursor = index === this.cursor && this.mode === "list" ? "\u25b6" : " ";
      const preview = mailPreview(mail, 16);
      tilemap.writeText(listLeft + 1, listTop + 1 + index, `${cursor}${String(index + 1).padStart(2, " ")}: ${preview}`, {
        maxLength: 18,
        pad: true,
      });
    });
    if (this.mode === "actions") {
      this.drawActionMenu(tilemap, height);
    }
    this.drawPreviewWindow(tilemap, height);
    blitPcTilemap(this.ui.screen, this.ui.font, tilemap);
    this.renderTextSnapshot(height);
  }

  private renderTextSnapshot(height: number): void {
    if (!this.ui?.renderSnapshot) {
      return;
    }
    const visible = this.messages.slice(0, height - 2);
    const viewportLines = ["MAIL BOX"];
    if (visible.length) {
      visible.forEach((mail, index) => {
        const cursor = index === this.cursor && this.mode === "list" ? "▶" : " ";
        viewportLines.push(`${cursor} ${String(index + 1).padStart(2, " ")}: ${mailPreview(mail, 16)}`);
      });
    } else {
      viewportLines.push("  NO MAIL");
    }
    const menuLines = this.mode === "actions"
      ? MAILBOX_ACTIONS.map((label, index) => `${index === this.actionIndex ? "▶" : " "} ${label}`)
      : null;
    this.ui.renderSnapshot(
      viewportLines,
      ["D-Pad=Move A=Select B=Back"],
      "Mailbox",
      "Legend",
      menuLines,
      this.currentPreview().split("\n").slice(0, 2),
      null,
    );
  }

  private drawPreviewWindow(tilemap: TilemapSurface, height: number): void {
    if (!this.ui) {
      return;
    }
    const previewLeft = 1;
    const previewTop = height + 1;
    tilemap.drawWindow(previewLeft, previewTop, 20, MAILBOX_PREVIEW_HEIGHT, {
      attr: PC_TEXT_PALETTE,
      fillTile: SPACE_TILE,
    });
    const preview = this.currentPreview().split("\n").slice(0, 2);
    preview.forEach((line, row) => {
      tilemap.writeText(previewLeft + 1, previewTop + 1 + row, line, { maxLength: 18, pad: true });
    });
  }

  private currentPreview(): string {
    const entry = this.getMailEntry(this.cursor);
    return entry ? entry.message : "(empty)";
  }

  private getMailEntry(index: number): MailMessage | null {
    if (!this.messages.length) {
      return null;
    }
    if (index >= this.messages.length) {
      this.cursor = Math.max(0, this.messages.length - 1);
      index = this.cursor;
    }
    if (index >= 0 && index < this.messages.length) {
      return this.messages[index];
    }
    return null;
  }

  private drawActionMenu(tilemap: TilemapSurface, height: number): void {
    if (!this.ui) {
      return;
    }
    const menuLeft = 16;
    const menuTop = height;
    const menuWidth = 6;
    tilemap.drawWindow(menuLeft, menuTop, menuWidth, MAILBOX_ACTIONS.length + 2, {
      attr: PC_TEXT_PALETTE,
      fillTile: SPACE_TILE,
    });
    MAILBOX_ACTIONS.forEach((label, index) => {
      const cursor = index === this.actionIndex ? "\u25b6" : " ";
      tilemap.writeText(menuLeft + 1, menuTop + 1 + index, `${cursor}${label}`, {
        maxLength: menuWidth - 2,
        pad: true,
      });
    });
  }

  handleInput(event: KeyEvent): Record<string, unknown> | null {
    if (!isKeyDownEvent(event) || !this.ui?.screen) {
      return null;
    }
    if (this.mode === "list") {
      return this.handleListInput(event);
    }
    return this.handleActionInput(event);
  }

  private handleListInput(event: KeyEvent): Record<string, unknown> | null {
    const key = event.key;
    if (key === gameEngine.K_UP) {
      this.cursor = Math.max(0, this.cursor - 1);
      this.playCursor();
    } else if (key === gameEngine.K_DOWN) {
      this.cursor = Math.min(Math.max(0, this.messages.length - 1), this.cursor + 1);
      this.playCursor();
    } else if (isConfirmEvent(event)) {
      if (this.messages.length) {
        this.mode = "actions";
        this.actionIndex = 0;
        this.playConfirm();
      }
    } else if (isCancelEvent(event)) {
      this.playConfirm();
      return { action: "exit" };
    }
    return null;
  }

  private handleActionInput(event: KeyEvent): Record<string, unknown> | null {
    const key = event.key;
    if (key === gameEngine.K_UP || key === gameEngine.K_LEFT) {
      this.actionIndex = (this.actionIndex - 1 + MAILBOX_ACTIONS.length) % MAILBOX_ACTIONS.length;
      this.playCursor();
      return null;
    }
    if (key === gameEngine.K_DOWN || key === gameEngine.K_RIGHT) {
      this.actionIndex = (this.actionIndex + 1) % MAILBOX_ACTIONS.length;
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
    const action = MAILBOX_ACTIONS[this.actionIndex].toLowerCase();
    this.mode = "list";
    this.playConfirm();
    if (action === "cancel") {
      return null;
    }
    const currentMessage = this.getMailEntry(this.cursor)?.message ?? "";
    return {
      action,
      index: this.cursor,
      ...(action === "read" ? { message: currentMessage } : {}),
    };
  }

  scriptedActions(actions: Iterable<Record<string, unknown>> | null): Array<Record<string, unknown>> {
    const results: Array<Record<string, unknown>> = [];
    if (!actions) {
      return results;
    }
    for (const entry of actions) {
      if (!entry || typeof entry !== "object") {
        results.push({ status: "invalid" });
        continue;
      }
      results.push(this.executeAction(entry));
    }
    return results;
  }

  runInteractive(opts: {
    handler: (action: Record<string, unknown>) => Record<string, unknown> | null;
    drawCallback?: () => void;
  }): Array<Record<string, unknown>> {
    if (!this.ui?.screen || !this.ui.eventQueue) {
      return [];
    }
    const results: Array<Record<string, unknown>> = [];
    let running = true;
    while (running) {
      for (const event of gameEngine.event.get(this.ui.eventQueue)) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("Quit requested.");
        }
        const action = this.handleInput(event);
        if (!action) {
          continue;
        }
        if (action.action === "exit") {
          running = false;
          break;
        }
        const response = opts.handler(action);
        if (response) {
          results.push(response);
        }
      }
      opts.drawCallback?.();
      this.draw();
      this.ui.update?.();
    }
    return results;
  }

  async runInteractiveAsync(opts: {
    handler: (action: Record<string, unknown>) => Record<string, unknown> | null;
    drawCallback?: () => void;
  }): Promise<Array<Record<string, unknown>>> {
    if (!this.ui?.screen) {
      return [];
    }
    const results: Array<Record<string, unknown>> = [];
    let running = true;
    while (running) {
      for (const event of gameEngine.event.get(this.ui?.eventQueue)) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("Quit requested.");
        }
        const action = this.handleInput(event);
        if (!action) {
          continue;
        }
        if (action.action === "exit") {
          running = false;
          break;
        }
        const response = opts.handler(action);
        if (response) {
          results.push(response);
        }
      }
      opts.drawCallback?.();
      this.draw();
      this.ui.update?.();
      await nextFrame();
    }
    return results;
  }

  executeAction(entry: Record<string, unknown>): Record<string, unknown> {
    const action = String(entry.action ?? "").toLowerCase();
    const index = Math.max(0, Number(entry.index ?? 0));
    if (action === "read") {
      const mailEntry = this.getMailEntry(index);
      return { action: "read", index, message: mailEntry?.message ?? "" };
    }
    if (action === "take") {
      const mailEntry = this.getMailEntry(index);
      if (mailEntry) {
        const removed = this.messages.splice(index, 1)[0];
        return { action: "take", index, message: removed.message, status: "ok" };
      }
      return { action: "take", index, status: "out_of_range" };
    }
    if (action === "delete") {
      const mailEntry = this.getMailEntry(index);
      if (mailEntry) {
        const removed = this.messages.splice(index, 1)[0];
        return { action: "delete", index, message: removed.message, status: "ok" };
      }
      return { action: "delete", index, status: "out_of_range" };
    }
    if (action === "give" || action === "add") {
      const message = String(entry.message ?? "").trim();
      if (!message) {
        return { action, status: action === "give" ? "no_message" : "empty" };
      }
      const status = this.appendMailEntry(message);
      return { action, status };
    }
    return { status: "unknown_action", action };
  }

  private appendMailEntry(message: string): string {
    if (this.messages.length >= MAILBOX_CAPACITY) {
      return "full";
    }
    const mailEntry: MailMessage = {
      message,
      author: "PLAYER",
      nationality: MailLanguage.ENGLISH,
      author_id: 0,
      species_id: 0,
      mail_type: ItemEnum.FLOWER_MAIL,
    };
    this.messages.push(mailEntry);
    return "ok";
  }

  private playCursor(): void {}

  private playConfirm(): void {
    this.audioEngine?.playSound("menu_option");
  }
}

export class PokedexRatingScreen {
  constructor(private readonly ui: MenuUI | null, private readonly audioEngine?: AudioEngine | null) {}

  draw(message: string): void {
    if (!this.ui?.screen) {
      return;
    }
    this.ui.screen.fill([255, 255, 255, 255]);
    const { tilemap } = createPcTilemap();
    const width = 20;
    const height = 6;
    tilemap.drawWindow(1, 1, width, height, { attr: PC_TEXT_PALETTE, fillTile: SPACE_TILE });
    const lines = (message || "").split("\n");
    lines.slice(0, height - 2).forEach((line, index) => {
      tilemap.writeText(2, 2 + index, line, { maxLength: width - 2, pad: true });
    });
    blitPcTilemap(this.ui.screen, this.ui.font, tilemap);
    this.ui.renderSnapshot?.(
      ["PROF.OAK'S PC"],
      ["A=Continue", ...lines],
      "Oak's PC",
      "Rating",
      null,
      null,
      null,
    );
  }
}

export class HallOfFameViewer {
  private playCursor(): void {
    if (this.audioEngine?.play_sound) {
      this.audioEngine.play_sound("cursor");
    }
  }

  constructor(private readonly ui: MenuUI | null, private readonly audioEngine?: AudioEngine | null) {}

  draw(entry: string[], index: number): void {
    if (!this.ui?.screen) {
      return;
    }
    this.ui.screen.fill([255, 255, 255, 255]);
    const { tilemap } = createPcTilemap();
    const title = `HALL OF FAME #${String(index + 1).padStart(2, "0")}`;
    const width = 20;
    const height = Math.max(6, entry.length + 3);
    tilemap.drawWindow(1, 1, width, height, { attr: PC_TEXT_PALETTE, fillTile: SPACE_TILE });
    tilemap.writeText(2, 2, title, { maxLength: width - 2, pad: true });
    entry.slice(0, height - 3).forEach((name, offset) => {
      const label = name || "-----";
      tilemap.writeText(2, 3 + offset, `${String(offset + 1).padStart(2, " ")}. ${label}`, {
        maxLength: width - 2,
        pad: true,
      });
    });
    blitPcTilemap(this.ui.screen, this.ui.font, tilemap);
    this.ui.renderSnapshot?.(
      [title, ...entry.map((name, offset) => `${String(offset + 1).padStart(2, " ")}. ${name || "-----"}`)],
      ["Left/Right=Entry A/B=Back"],
      "Hall of Fame",
      "Legend",
      null,
      null,
      null,
    );
  }

  runInteractive(entries: string[][], drawCallback?: () => void): Record<string, unknown> {
    const summary: Record<string, unknown> = { entries: entries.map((entry) => [...entry]) };
    if (!entries.length || !this.ui?.screen || !this.ui.eventQueue) {
      summary.status = "empty";
      return summary;
    }
    let index = 0;
    let running = true;
    while (running) {
      for (const event of gameEngine.event.get(this.ui.eventQueue)) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("Quit requested.");
        }
        if (!isKeyDownEvent(event)) {
          continue;
        }
        if (event.key === gameEngine.K_LEFT || event.key === gameEngine.K_UP) {
          index = (index - 1 + entries.length) % entries.length;
          this.audioEngine?.playSound("menu_option");
        } else if (event.key === gameEngine.K_RIGHT || event.key === gameEngine.K_DOWN) {
          index = (index + 1) % entries.length;
          this.audioEngine?.playSound("menu_option");
        } else if (isCancelEvent(event) || isConfirmEvent(event)) {
          this.playConfirm();
          running = false;
          break;
        }
      }
      drawCallback?.();
      this.draw(entries[index], index);
      this.ui.update?.();
    }
    summary.selected_index = index;
    summary.selected = [...entries[index]];
    return summary;
  }

  async runInteractiveAsync(entries: string[][], drawCallback?: () => void): Promise<Record<string, unknown>> {
    const summary: Record<string, unknown> = { entries: entries.map((entry) => [...entry]) };
    if (!entries.length || !this.ui?.screen) {
      summary.status = "empty";
      return summary;
    }
    let index = 0;
    let running = true;
    while (running) {
      for (const event of gameEngine.event.get(this.ui?.eventQueue)) {
        if (event.type === gameEngine.QUIT) {
          gameEngine.quit();
          throw new Error("Quit requested.");
        }
        if (!isKeyDownEvent(event)) {
          continue;
        }
        if (event.key === gameEngine.K_LEFT || event.key === gameEngine.K_UP) {
          index = (index - 1 + entries.length) % entries.length;
          this.audioEngine?.playSound("menu_option");
        } else if (event.key === gameEngine.K_RIGHT || event.key === gameEngine.K_DOWN) {
          index = (index + 1) % entries.length;
          this.audioEngine?.playSound("menu_option");
        } else if (isCancelEvent(event) || isConfirmEvent(event)) {
          this.playConfirm();
          running = false;
          break;
        }
      }
      drawCallback?.();
      this.draw(entries[index], index);
      this.ui.update?.();
      await nextFrame();
    }
    summary.selected_index = index;
    summary.selected = [...entries[index]];
    return summary;
  }

  private playConfirm(): void {
    this.audioEngine?.playSound("menu_option");
  }
}
