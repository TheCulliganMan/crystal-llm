// ASM: engine/items/pack.asm
import { MenuUI } from "./types";
import { GameState } from "../../core/state";
import { AudioEngine } from "../../engine/systems/audio";
import { ItemPocket } from "../../core/enums/item";
import { ItemSystem } from "../../engine/systems/items";
import * as tmhmSystem from "../../engine/systems/tmhm";
import { gameEngine } from "../game-engine";
import type { ItemSystemDataLoader } from "../../engine/systems/items";
import {
  KeyEvent,
  isCancelEvent,
  isConfirmEvent,
  isKeyDownEvent,
  isSelectEvent,
  isStartEvent,
} from "../../input/buttons";
import { BGMapWriter } from "../bg-map-sync";
import {
  BagMenuTilemap,
  actionWindowLeft,
  actionWindowWidth,
  bagTileset,
  bottomTextboxRegion,
  itemTextRegion,
  labelColumn,
  pointerColumn,
  quantityColumn,
  scrollColumn,
  seedTilemapBase,
} from "./bag-menu-layout";
import { TileRegion } from "../tile-layout";
import type { TilemapTileset } from "../tilemap-surface";
import { PlayerGender } from "../../core/enums";

const LIST_HEIGHT = 7;
const ACTION_MENU_TOPS: Record<number, number> = {
  5: 1,
  4: 3,
  3: 5,
  2: 7,
  1: 9,
};

const AUTO_INPUT = 0xff;

type PocketStorage = Record<string, number> | number[];

class Pocket {
  constructor(
    public readonly label: string,
    public readonly pocket: ItemPocket,
    public readonly storage: PocketStorage,
  ) {}

  items(): Array<[string, number]> {
    if (Array.isArray(this.storage)) {
      return [];
    }
    return Object.entries(this.storage).filter(([, qty]) => qty > 0) as Array<[string, number]>;
  }
}

export class BagMenu {
  private readonly screen;
  private readonly pockets: Pocket[];
  private readonly bgMapWriter: BGMapWriter;
  private readonly itemSystem: ItemSystem;
  private readonly tutorialBagActive: boolean;
  private pocketIndex = 0;
  private listIndex = 0;
  private scrollOffset = 0;
  private mode: "list" | "actions" = "list";
  private actionIndex = 0;
  private actionOptions: string[] = [];
  private forcedActionOptions: string[] | null = null;

  constructor(
    private readonly ui: MenuUI,
    private readonly gameState: GameState,
    private readonly audioEngine?: AudioEngine | null,
    dataLoader?: ItemSystemDataLoader,
    actionOptions?: string[],
  ) {
    this.screen = ui.screen;
    const sram = this.gameState.sram;
    this.tutorialBagActive = this.isTutorialBattle();
    const wram = this.gameState.wram;
    const itemStorage = this.tutorialBagActive ? wram.wDudeItems : sram.items;
    const ballStorage = this.tutorialBagActive ? wram.wDudeBalls : sram.balls;
    const keyStorage = this.tutorialBagActive ? wram.wDudeKeyItems : sram.key_items;
    this.pockets = [
      new Pocket("ITEMS", ItemPocket.ITEM, itemStorage),
      new Pocket("BALL", ItemPocket.BALL, ballStorage),
      new Pocket("KEY", ItemPocket.KEY_ITEM, keyStorage),
      new Pocket("TM/HM", ItemPocket.TM_HM, sram.tm_hm),
    ];
    this.bgMapWriter = new BGMapWriter(this.gameState, "vBGMap0");
    this.itemSystem = new ItemSystem(this.gameState, dataLoader);
    if (actionOptions) {
      this.forcedActionOptions = actionOptions.map((option) => option.toUpperCase());
    }
  }

  getUi(): MenuUI {
    return this.ui;
  }

  getGameState(): GameState {
    return this.gameState;
  }

  getPocketIndex(): number {
    return this.pocketIndex;
  }

  getListIndex(): number {
    return this.listIndex;
  }

  getScrollOffset(): number {
    return this.scrollOffset;
  }

  getMode(): "list" | "actions" {
    return this.mode;
  }

  getActionIndex(): number {
    return this.actionIndex;
  }

  getActionOptions(): string[] {
    return [...this.actionOptions];
  }

  getCurrentPocketLabel(): string {
    return this.currentPocket().label;
  }

  getVisibleItems(): Array<[string, number]> {
    return this.visibleItems();
  }

  getCurrentItems(): Array<[string, number]> {
    return this.currentItems();
  }

  static layoutRegions(): Record<string, TileRegion> {
    return {
      pocket_panel: new TileRegion(0, 0, 5, 10),
      list: itemTextRegion(),
      textbox: bottomTextboxRegion(),
      action: new TileRegion(actionWindowLeft(), 3, actionWindowWidth(), 6),
    };
  }

  private currentPocket(): Pocket {
    return this.pockets[this.pocketIndex];
  }

  private currentItems(): Array<[string, number]> {
    const pocket = this.currentPocket();
    let items: Array<[string, number]>;
    if (pocket.pocket === ItemPocket.TM_HM) {
      items = this.tmhmItems();
    } else {
      items = pocket.items();
    }
    // ASM: engine/menus/scrolling_menu.asm::ScrollingMenu_GetListItemCoordAndFunctionArgs
    // The menu renders a trailing CANCEL row from the -1 terminator; model it explicitly.
    items.push(["CANCEL", 0]);
    return items;
  }

  private visibleItems(): Array<[string, number]> {
    const items = this.currentItems();
    return items.slice(this.scrollOffset, this.scrollOffset + LIST_HEIGHT);
  }

  private playerGender(): PlayerGender {
    if (this.tutorialBagActive) {
      return PlayerGender.MALE;
    }
    const gender = this.gameState.wram.player_gender;
    return typeof gender === "number" ? (gender as PlayerGender) : PlayerGender.MALE;
  }

  handleInput(event: KeyEvent): [string, string] | null {
    if (!isKeyDownEvent(event)) {
      return null;
    }
    if (this.mode === "list") {
      return this.handleListInput(event);
    }
    return this.handleActionInput(event);
  }

  private handleListInput(event: KeyEvent): [string, string] | null {
    const items = this.currentItems();
    const key = event.key;
    if (key === gameEngine.K_UP) {
      if (items.length > 0) {
        this.listIndex = (this.listIndex - 1 + items.length) % items.length;
        this.clampCursor();
        this.playCursor();
      }
    } else if (key === gameEngine.K_DOWN) {
      if (items.length > 0) {
        this.listIndex = (this.listIndex + 1) % items.length;
        this.clampCursor();
        this.playCursor();
      }
    } else if (key === gameEngine.K_LEFT || key === gameEngine.K_q) {
      this.shiftPocket(-1);
    } else if (key === gameEngine.K_RIGHT || key === gameEngine.K_e) {
      this.shiftPocket(1);
    } else if (isConfirmEvent(event)) {
      if (items.length > 0) {
        const [name] = items[this.listIndex];
        if (name === "CANCEL") {
          return ["cancel", ""];
        }
        if (this.isAutoTutorial()) {
          this.playConfirm();
          return ["use", name];
        }
        this.openActionMenu();
      }
    } else if (isSelectEvent(event)) {
      if (items.length > 0) {
        const [name] = items[this.listIndex];
        if (name === "CANCEL") {
          return ["cancel", ""];
        }
        if (this.currentPocket().pocket === ItemPocket.KEY_ITEM) {
          this.playConfirm();
          return ["sel", name];
        }
      }
    } else if (isStartEvent(event)) {
      if (items.length > 0) {
        const [name] = items[this.listIndex];
        if (name === "CANCEL") {
          return ["cancel", ""];
        }
      }
    } else if (isCancelEvent(event)) {
      this.playConfirm();
      return ["cancel", ""];
    }
    return null;
  }

  private handleActionInput(event: KeyEvent): [string, string] | null {
    if (this.actionOptions.length === 0) {
      this.mode = "list";
      return null;
    }
    const key = event.key;
    if (key === gameEngine.K_UP || key === gameEngine.K_LEFT) {
      this.actionIndex = (this.actionIndex - 1 + this.actionOptions.length) % this.actionOptions.length;
      this.playCursor();
      return null;
    }
    if (key === gameEngine.K_DOWN || key === gameEngine.K_RIGHT) {
      this.actionIndex = (this.actionIndex + 1) % this.actionOptions.length;
      this.playCursor();
      return null;
    }
    if (isCancelEvent(event)) {
      this.mode = "list";
      this.playConfirm();
      return null;
    }
    if (isConfirmEvent(event)) {
      const action = this.actionOptions[this.actionIndex];
      const items = this.currentItems();
      if (!items.length) {
        this.mode = "list";
        return null;
      }
      const [itemName] = items[this.listIndex];
      if (itemName === "CANCEL") {
        this.mode = "list";
        return ["cancel", ""];
      }
      this.mode = "list";
      this.playConfirm();
      if (action === "QUIT") {
        return null;
      }
      if (action === "TOSS") {
        this.tossItem(itemName);
        return ["toss", itemName];
      }
      return [action.toLowerCase(), itemName];
    }
    return null;
  }

  private openActionMenu(): void {
    if (this.forcedActionOptions) {
      this.actionOptions = [...this.forcedActionOptions];
    } else {
      const pocket = this.currentPocket().pocket;
      if (pocket === ItemPocket.KEY_ITEM) {
        this.actionOptions = ["USE", "SEL", "QUIT"];
      } else if (pocket === ItemPocket.TM_HM) {
        this.actionOptions = ["USE", "QUIT"];
      } else {
        this.actionOptions = ["USE", "GIVE", "TOSS", "QUIT"];
      }
    }
    this.mode = "actions";
    this.actionIndex = 0;
    this.playConfirm();
  }

  private shiftPocket(delta: number): void {
    this.persistTmhmCursor();
    const total = this.pockets.length;
    this.pocketIndex = (this.pocketIndex + delta + total) % total;
    this.audioEngine?.playSound("SFX_SWITCH_POCKETS");
    if (this.currentPocket().pocket === ItemPocket.TM_HM) {
      this.restoreTmhmCursor();
    } else {
      this.listIndex = 0;
      this.scrollOffset = 0;
    }
  }

  private isAutoTutorial(): boolean {
    const { wram } = this.gameState;
    return this.isTutorialBattle() || Number(wram.wInputType ?? 0) === AUTO_INPUT;
  }

  private isTutorialBattle(): boolean {
    const battleType = String(this.gameState.wram.battle_type ?? "").toUpperCase();
    return battleType === "BATTLETYPE_TUTORIAL";
  }

  private clampCursor(): void {
    const items = this.currentItems();
    if (!items.length) {
      this.listIndex = 0;
      this.scrollOffset = 0;
      return;
    }
    this.listIndex = Math.max(0, Math.min(this.listIndex, items.length - 1));
    if (this.listIndex < this.scrollOffset) {
      this.scrollOffset = this.listIndex;
    } else if (this.listIndex >= this.scrollOffset + LIST_HEIGHT) {
      this.scrollOffset = this.listIndex - LIST_HEIGHT + 1;
    }
  }

  private playCursor(): void {}

  private playConfirm(): void {
    this.audioEngine?.playSound("menu_option");
  }

  private tossItem(itemName: string): void {
    const pocket = this.currentPocket().storage;
    if (Array.isArray(pocket)) {
      return;
    }
    const qty = pocket[itemName] ?? 0;
    if (qty <= 0) {
      return;
    }
    if (qty === 1) {
      delete pocket[itemName];
    } else {
      pocket[itemName] = qty - 1;
    }
    this.clampCursor();
  }

  draw(): void {
    const screen = this.screen;
    if (!screen) {
      throw new Error("UI screen surface is not initialised.");
    }
    const tilemap = this.buildTilemap();
    const tileset: TilemapTileset = bagTileset(this.ui.font, this.pocketIndex, this.playerGender());
    tilemap.blit(screen, tileset);
    this.bgMapWriter.request(tilemap);
  }

  private buildTilemap(): BagMenuTilemap {
    this.clampCursor();
    const tilemap = new BagMenuTilemap();
    seedTilemapBase(tilemap, this.pocketIndex);
    this.writeItemList(tilemap);
    this.writeDescription(tilemap);
    if (this.mode === "actions") {
      this.writeActionMenu(tilemap);
    }
    return tilemap;
  }

  private writeItemList(tilemap: BagMenuTilemap): void {
    const region = itemTextRegion();
    const pointerCol = pointerColumn();
    const nameCol = labelColumn();
    const qtyCol = quantityColumn();
    const arrowCol = scrollColumn();
    const visible = this.visibleItems();
    const fullItems = this.currentItems();
    const usesQuantityColumn = this.currentPocket().pocket !== ItemPocket.KEY_ITEM;
    const labelWidth = usesQuantityColumn ? qtyCol - nameCol : arrowCol - nameCol + 1;
    for (let index = 0; index < LIST_HEIGHT; index += 1) {
      const row = region.top + index;
      let pointer = " ";
      let label = "";
      let qtyText = "";
      const absoluteIndex = this.scrollOffset + index;
      if (index < visible.length) {
        const [name, qty] = visible[index];
        pointer = absoluteIndex === this.listIndex && fullItems.length ? "\u25b6" : " ";
        const display = this.itemSystem.getDisplayName(name).toUpperCase();
        label = display.slice(0, labelWidth);
        qtyText = usesQuantityColumn && name !== "CANCEL" ? `\u00d7${String(qty).padStart(2, "0")}` : "";
      }
      tilemap.writeText(pointerCol, row, pointer, { maxLength: 1, pad: false });
      tilemap.writeText(nameCol, row, label, { maxLength: labelWidth, pad: true });
      if (usesQuantityColumn) {
        tilemap.writeText(qtyCol, row, qtyText, { maxLength: 3, pad: true });
      }
    }
    if (this.scrollOffset > 0) {
      tilemap.writeText(arrowCol, region.top, "\u25b2", { maxLength: 1, pad: false });
    }
    if (this.scrollOffset + LIST_HEIGHT < fullItems.length) {
      tilemap.writeText(arrowCol, region.top + LIST_HEIGHT - 1, "\u25bc", {
        maxLength: 1,
        pad: false,
      });
    }
    if (fullItems.length === 0) {
      tilemap.writeText(nameCol, region.top, "NO ITEMS", { maxLength: labelWidth, pad: false });
    }
  }

  private writeDescription(tilemap: BagMenuTilemap): void {
    const region = bottomTextboxRegion();
    const innerWidth = region.width - 2;
    const textTop = region.top + 2;
    const maxLines = region.height - 3;
    const lines = this.descriptionLines(innerWidth, maxLines);
    for (let offset = 0; offset < region.height - 2; offset += 1) {
      tilemap.writeText(region.left + 1, region.top + 1 + offset, "", {
        maxLength: innerWidth,
        pad: true,
      });
    }
    for (let offset = 0; offset < maxLines; offset += 1) {
      const text = offset < lines.length ? lines[offset] : "";
      tilemap.writeText(region.left + 1, textTop + offset, text, {
        maxLength: innerWidth,
        pad: true,
      });
    }
  }

  private descriptionLines(width: number, maxLines: number): string[] {
    const items = this.currentItems();
    if (!items.length) {
      const pocketLabel = `${this.currentPocket().label} POCKET IS EMPTY.`;
      return this.wrapText(pocketLabel, width, maxLines);
    }
    const [name] = items[this.listIndex];
    if (name === "CANCEL") {
      return [];
    }
    const definition = this.itemSystem.getItemDefinition(name);
    const description = definition.description || "NO DESCRIPTION AVAILABLE.";
    return this.wrapText(description, width, maxLines);
  }

  private wrapText(text: string, width: number, maxLines: number): string[] {
    const normalized = text.replace(/\n/g, " ").replace(/  +/g, " ").trim();
    if (!normalized) {
      return [];
    }
    const splitWord = (word: string): string[] => {
      if (word.length <= width) {
        return [word];
      }
      const pieces: string[] = [];
      const chunks = word.split("-");
      if (chunks.length > 1) {
        chunks.forEach((chunk, index) => {
          const token = index < chunks.length - 1 ? `${chunk}-` : chunk;
          if (token.length <= width) {
            pieces.push(token);
          } else {
            for (let offset = 0; offset < token.length; offset += width) {
              pieces.push(token.slice(offset, offset + width));
            }
          }
        });
        return pieces;
      }
      for (let offset = 0; offset < word.length; offset += width) {
        pieces.push(word.slice(offset, offset + width));
      }
      return pieces;
    };

    const tokens: string[] = [];
    normalized.split(" ").forEach((word) => {
      tokens.push(...splitWord(word));
    });

    const lines: string[] = [];
    let current = "";
    for (const token of tokens) {
      if (!current) {
        current = token;
        continue;
      }
      if (`${current} ${token}`.length <= width) {
        current = `${current} ${token}`;
      } else {
        lines.push(current);
        current = token;
      }
    }
    if (current) {
      lines.push(current);
    }
    if (lines.length <= maxLines) {
      return lines;
    }
    const trimmed = lines.slice(0, maxLines);
    trimmed[maxLines - 1] = trimmed[maxLines - 1].slice(0, Math.max(0, width - 1)).trimEnd() + "\u2026";
    return trimmed;
  }

  private writeActionMenu(tilemap: BagMenuTilemap): void {
    if (!this.actionOptions.length) {
      return;
    }
    const width = actionWindowWidth();
    const top = ACTION_MENU_TOPS[this.actionOptions.length] ?? 7;
    tilemap.drawWindow(actionWindowLeft(), top, width, this.actionOptions.length + 2, {
      attr: 0x07,
    });
    const pointerCol = actionWindowLeft() + 1;
    const textCol = pointerCol + 1;
    const innerWidth = width - 3;
    this.actionOptions.forEach((label, idx) => {
      const row = top + 1 + idx;
      const pointer = idx === this.actionIndex ? "\u25b6" : " ";
      tilemap.writeText(pointerCol, row, pointer, { maxLength: 1, pad: false });
      tilemap.writeText(textCol, row, label, { maxLength: innerWidth, pad: true });
    });
  }

  private persistTmhmCursor(): void {
    if (this.currentPocket().pocket !== ItemPocket.TM_HM) {
      return;
    }
    this.gameState.wram.wTMHMPocketCursor = this.listIndex;
    this.gameState.wram.wTMHMPocketScrollPosition = this.scrollOffset;
  }

  private restoreTmhmCursor(): void {
    this.listIndex = this.gameState.wram.wTMHMPocketCursor;
    this.scrollOffset = this.gameState.wram.wTMHMPocketScrollPosition;
  }

  private tmhmItems(): Array<[string, number]> {
    if (this.tutorialBagActive) {
      return [];
    }
    const flags = this.gameState.sram.tm_hm;
    const required = tmhmSystem.TMHM_MOVES.length;
    while (flags.length < required) {
      flags.push(0);
    }
    const items: Array<[string, number]> = [];
    flags.forEach((enabled, index) => {
      if (enabled) {
        items.push([tmhmSystem.tmhmItemName(index), 1]);
      }
    });
    return items;
  }
}
