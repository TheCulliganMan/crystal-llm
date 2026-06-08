// ASM mapping: pokecrystal_disassembly/engine/menus/bills_pc.asm (BillsPC main loop and menu states).
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import {
  GameButton,
  KeyEvent,
  buttonKeys,
  isCancelEvent,
  isConfirmEvent,
  isKeyDownEvent,
  normalizeButtonKey,
} from "@pokecrystal/core/input/buttons";
import { saveGame } from "@pokecrystal/core/core/save";
import { Box, BoxSchema, formatDefaultBoxName, Party, Pokemon, toPokemon } from "@pokecrystal/core/core/models";
import { MAX_PC_BOXES } from "@pokecrystal/core/core/constants";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { GameState } from "@pokecrystal/core/core/state";
import { Surface, Rect } from "@pokecrystal/core/ui/surface";
import type { Palette, RenderTextOptions, SurfaceLike } from "@pokecrystal/core/ui/font-renderer";
import type { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import { isPromptUI, SelectionPrompt } from "@pokecrystal/core/ui/text/prompts";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { POKEMON_WORD, LV_GLYPH } from "@pokecrystal/core/ui/text/constants";
import { renderFontText } from "@pokecrystal/core/ui/text/render-font";
import { TilemapSurface, SPACE_TILE } from "@pokecrystal/core/ui/tilemap-surface";
import {
  blitPcTilemap,
  createPcTilemap,
  PC_TEXT_PALETTE,
} from "./pc-wallpaper";
import {
  BOTTOM_PROMPT_REGION,
  GRID_REGION,
  HEADER_REGION,
  INFO_CLEAR_REGION,
  INFO_GENDER_ORIGIN,
  INFO_ITEM_ORIGIN,
  INFO_LEVEL_ORIGIN,
  INFO_NAME_ORIGIN,
  INFO_PIC_ORIGIN,
  PC_LAYOUT,
} from "./pc-layout";
import {
  BillsPCListView,
  BILLS_PC_LIST_NAME_MAX_CHARS,
  BillsPCCursorView,
  PC_WINDOW_FILL,
  PCActionMenuView,
  PCActionNavigator,
  PCBottomPromptView,
  PCDepositNavigator,
  PCMessageWindowView,
  PCMoveNavigator,
  playPcSwitchSound,
} from "./pc-views";
import { PokemonStatsScreen, StatsUI } from "./pokemon-stats";

const TILE_SIZE = 8;

const DEPOSIT_ACTIONS = ["DEPOSIT", "STATS", "RELEASE", "CANCEL"] as const;
const WITHDRAW_ACTIONS = ["WITHDRAW", "STATS", "RELEASE", "CANCEL"] as const;
const INFO_NAME_MAX_CHARS = 10;
const MAIL_ITEM_NAMES = new Set([
  "FLOWER_MAIL",
  "SURF_MAIL",
  "LITEBLUEMAIL",
  "PORTRAITMAIL",
  "LOVELY_MAIL",
  "EON_MAIL",
  "MORPH_MAIL",
  "BLUESKY_MAIL",
  "MUSIC_MAIL",
  "MIRAGE_MAIL",
]);

export enum PCMode {
  BROWSE = "browse",
  ACTIONS = "actions",
  MOVE = "move",
  DEPOSIT = "deposit",
}

const isMailItem = (itemName?: string | null): boolean => {
  if (!itemName) {
    return false;
  }
  return MAIL_ITEM_NAMES.has(itemName.toUpperCase());
};


const BILLS_PC_ROWS = 5;
const BILLS_PC_PLACEHOLDER = "-----";
const BILLS_PC_CANCEL_LABEL = "CANCEL";
const BILLS_PC_CURRENT_BOX_VALUE = MAX_PC_BOXES + 1;
const MOVE_ACTIONS = ["MOVE", "STATS", "CANCEL"] as const;

enum BillsPCState {
  INIT,
  HANDLE_JOYPAD,
  WHATS_UP,
  SUBMENU,
  END_LOOP,
}

type BillTopLevelAction = "withdraw" | "deposit" | "move";

export interface BillsPCEntry {
  speciesId: string | null;
  sourceBoxValue: number;
  boxIndex: number | null;
  slotIndex: number;
  nickname: string;
  pokemon: Pokemon | null;
}

export interface PCFont {
  paletteVariants: (paletteOrder: ReadonlyArray<Palette>) => Record<number, Record<number, Surface>>;
  render_text?: (
    text: string,
    x: number,
    y: number,
    surface: Surface | SurfaceLike,
    options?: RenderTextOptions
  ) => void;
  renderText: (
    text: string,
    x: number,
    y: number,
    surface: Surface | SurfaceLike,
    options?: RenderTextOptions | boolean
  ) => void;
}

export interface SupportsPokemonPCUI {
  screen: Surface | null;
  font: PCFont;
  drawWindow: (
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    options?: { frameId?: number | null; fill?: [number, number, number] | null }
  ) => void;
  drawSprite: (
    speciesId: string,
    x: number,
    y: number,
    spriteType?: string,
    frame?: number
  ) => void;
  drawRect: (color: [number, number, number], rect: Rect, width?: number) => void;
  update: () => void;
  pollEvents?: () => KeyEvent[];
  renderSnapshot?: ScreenUI["renderSnapshot"];
}

export interface PCMenuActionPayload {
  action: string;
  box?: number | null;
  slot?: number | null;
  target_box?: number | null;
  target_slot?: number | null;
  party_slot?: number | null;
}

export interface PCMenuActionResult {
  action?: string;
  status?: string;
  box?: number;
  slot?: number;
  party_slot?: number;
  source_box?: number;
  source_slot?: number;
  target_box?: number;
  target_slot?: number;
  species?: string;
}

export type PCMenuInteractiveResponse = PCMenuActionPayload | PCMenuActionResult;

export class PokemonPCMenu {
  private static readonly PROMPT_LABELS: Record<PCMode, string> = {
    [PCMode.BROWSE]: "PCString_ChooseaPKMN",
    [PCMode.DEPOSIT]: "PCString_ChooseaPKMN",
    [PCMode.ACTIONS]: "PCString_WhatsUp",
    [PCMode.MOVE]: "PCString_MoveToWhere",
  };

  private readonly upKeys = new Set(["ArrowUp"]);
  private readonly downKeys = new Set(["ArrowDown"]);
  private readonly leftKeys = new Set(["ArrowLeft"]);
  private readonly rightKeys = new Set(["ArrowRight"]);
  private readonly confirmKeys = new Set(buttonKeys[GameButton.A]);
  private readonly cancelKeys = new Set(buttonKeys[GameButton.B]);

  private mode: PCMode = PCMode.BROWSE;
  private boxIndex: number;
  private actionIndex = 0;
  private moveOrigin: number | null = null;
  private moveOriginEntry: BillsPCEntry | null = null;
  private moveSessionActive = false;
  private pendingMovePayload: PCMenuActionPayload | null = null;
  private overlayMessage: string | null = null;
  private overlayFrames = 0;
  private readonly messageView = new PCMessageWindowView(HEADER_REGION);
  private readonly cursorView = new BillsPCCursorView(GRID_REGION, TILE_SIZE * 2, BILLS_PC_ROWS);
  private readonly listView = new BillsPCListView(
    GRID_REGION,
    BILLS_PC_ROWS,
    BILLS_PC_PLACEHOLDER,
    BILLS_PC_CANCEL_LABEL,
    this.cursorView
  );
  private readonly actionsView = new PCActionMenuView(PC_LAYOUT);
  private readonly bottomPromptView = new PCBottomPromptView(BOTTOM_PROMPT_REGION);
  private actionLabels: string[] = [...WITHDRAW_ACTIONS];
  private pendingEntry: BillsPCEntry | null = null;
  private readonly actionNav = new PCActionNavigator(this.actionLabels.length);
  private readonly moveNav = new PCMoveNavigator();
  private readonly depositNav: PCDepositNavigator;
  private billAction: BillTopLevelAction = "withdraw";
  private cursorPosition = 0;
  private scrollPosition = 0;
  private numMonsOnScreen = BILLS_PC_ROWS;
  private loadedBoxValue = 0;
  private monEntries: BillsPCEntry[] = [];
  private numMonsInBox = 1;
  private statsModalActive = false;
  private readonly statsScreen: PokemonStatsScreen;
  private jumptableState = BillsPCState.INIT;

  constructor(
    private readonly ui: SupportsPokemonPCUI,
    private readonly gameState: GameState,
    private readonly audioEngine: AudioEngine | null = null,
    private readonly dataLoader?: { getText?: (label: string) => string | null }
  ) {
    const boxes = this.boxes;
    const currentBox = boxes.length > 0 ? (this.gameState.sram.current_pc_box || 0) & 0x0f : 0;
    this.boxIndex = Math.max(0, Math.min(currentBox, Math.max(0, boxes.length - 1)));
    this.loadedBoxValue = this.normalizeLoadedBoxValue(this.boxIndex + 1);
    this.depositNav = new PCDepositNavigator(this.party.pokemon.length);
    const statsUi: StatsUI = {
      screen: this.ui.screen,
      font: this.ui.font,
    };
    this.statsScreen = new PokemonStatsScreen(statsUi, this.gameState);
    this.ensureMonList();
    this.setCurrentPcBox(this.boxIndex);
  }

  get boxes(): Box[] {
    return this.gameState.sram.pc_boxes;
  }

  get party(): Party {
    return this.gameState.sram.party;
  }

  setActiveBox(index: number): void {
    if (!this.boxes.length) {
      return;
    }
    const clamped = Math.max(0, Math.min(index, this.boxes.length - 1));
    this.boxIndex = clamped;
    this.loadedBoxValue = this.normalizeLoadedBoxValue(this.boxIndex + 1);
    this.setCurrentPcBox(clamped);
    this.resetCursor();
  }

  showBillAction(action: "withdraw" | "deposit" | "move"): void {
    this.billAction = action;
    if (action === "deposit") {
      this.loadedBoxValue = 0;
      this.mode = PCMode.DEPOSIT;
    } else if (action === "move") {
      this.loadedBoxValue = this.normalizeLoadedBoxValue(this.boxIndex + 1);
      this.mode = PCMode.BROWSE;
    } else {
      this.loadedBoxValue = BILLS_PC_CURRENT_BOX_VALUE;
      this.mode = PCMode.BROWSE;
    }
    this.syncBoxIndex();
    this.resetCursor();
    this.clearPendingEntry();
    this.ensureMonList();
  }

  private setCurrentPcBox(index: number): void {
    if (!this.boxes.length) {
      this.gameState.sram.current_pc_box = 0;
      return;
    }
    const clamped = Math.max(0, Math.min(Math.trunc(index) & 0x0f, this.boxes.length - 1));
    this.gameState.sram.current_pc_box = clamped;
  }

  startMoveSession(
    {
      confirmationProvider,
      drawCallback,
    }: { confirmationProvider?: () => boolean; drawCallback?: () => void } = {}
  ): boolean {
    if (this.anyPokemonHoldingMail()) {
      this.setOverlay(this.resolveOverlayText("PCString_RemoveMail", "Remove MAIL."), 60);
      return false;
    }
    const provider = confirmationProvider ?? (() => this.promptMoveWithoutMail(drawCallback));
    if (!provider()) {
      this.stopMoveSession();
      return false;
    }
    this.moveSessionActive = true;
    this.moveOrigin = null;
    this.moveOriginEntry = null;
    this.pendingMovePayload = null;
    return true;
  }

  stopMoveSession(): void {
    this.moveSessionActive = false;
    this.exitMoveMode();
  }

  handleInput(event: KeyEvent): [string, number | null, number] | null {
    if (this.statsModalActive) {
      const result = this.statsScreen.handleInput(event);
      if (result === "exit") {
        this.closeStatsScreen();
      }
      return null;
    }
    if (!isKeyDownEvent(event) || this.ui.screen === null) {
      return null;
    }
    const keyName = typeof event.key === "string" ? event.key : typeof event.code === "string" ? event.code : "";
    const keyCode = normalizeButtonKey(event.code ?? event.key ?? event.button ?? null);
    if (this.mode === PCMode.BROWSE) {
      this.ensureMonList();
      return this.handleBrowseInput(event, keyName, keyCode);
    }
    if (this.mode === PCMode.ACTIONS) {
      return this.handleActionMode(event, keyName, keyCode);
    }
    if (this.mode === PCMode.MOVE) {
      return this.handleMoveMode(event, keyName, keyCode);
    }
    if (this.mode === PCMode.DEPOSIT) {
      return this.handleDepositMode(event, keyName, keyCode);
    }
    return null;
  }

  private handleBrowseInput(event: KeyEvent, keyName: string, keyCode: number | null): [string, number | null, number] | null {
    let moved = false;
    if (this.upKeys.has(keyName)) {
      moved = this.pressUp();
    } else if (this.downKeys.has(keyName)) {
      moved = this.pressDown();
    } else if (this.leftKeys.has(keyName)) {
      moved = this.pressLeft();
    } else if (this.rightKeys.has(keyName)) {
      moved = this.pressRight();
    } else if (isConfirmEvent(event) || (keyCode !== null && this.confirmKeys.has(keyCode))) {
      if (this.selectionIsCancel()) {
        return ["cancel", -1, -1];
      }
      const entry = this.currentSelectionEntry();
      if (!entry) {
        return null;
      }
      if (this.billAction === "deposit" && entry.boxIndex === null) {
        this.mode = PCMode.BROWSE;
        this.playConfirm();
        this.clearPendingEntry();
        this.jumptableState = BillsPCState.END_LOOP;
        return ["deposit", this.boxIndex, entry.slotIndex];
      }
      this.enterActionMode(entry);
      return null;
    } else if (isCancelEvent(event) || (keyCode !== null && this.cancelKeys.has(keyCode))) {
      return ["cancel", -1, -1];
    }
    if (moved) {
      this.playCursor();
    }
    return null;
  }

  private handleActionMode(event: KeyEvent, keyName: string, keyCode: number | null): [string, number | null, number] | null {
    const [newIndex, navigatorAction] = this.actionNav.handleKey(keyName, keyCode, this.actionIndex);
    const action =
      navigatorAction ??
      (isConfirmEvent(event) ? "confirm" : isCancelEvent(event) ? "cancel" : null);
    this.actionIndex = newIndex;
    if (action === "cursor_move") {
      this.playCursor();
      return null;
    }
    if (action === "cancel") {
      this.mode = PCMode.BROWSE;
      this.clearPendingEntry();
      this.playCursor();
      this.jumptableState = BillsPCState.HANDLE_JOYPAD;
      return null;
    }
    if (action !== "confirm") {
      return null;
    }
    if (!this.actionLabels.length) {
      return null;
    }
    const index = Math.max(0, Math.min(this.actionIndex, this.actionLabels.length - 1));
    const label = this.actionLabels[index].toLowerCase();
    this.playConfirm();
    if (label === "cancel") {
      this.mode = PCMode.BROWSE;
      this.clearPendingEntry();
      return null;
    }
    if (label === "stats") {
      this.showStatsScreen();
      return null;
    }
    if (label === "move") {
      this.mode = PCMode.BROWSE;
      this.clearPendingEntry();
      this.beginMoveSelection();
      return null;
    }
    const entry = this.pendingEntry;
    if (!entry) {
      this.mode = PCMode.BROWSE;
      this.clearPendingEntry();
      return null;
    }
    if (label === "withdraw") {
      if (entry.boxIndex === null) {
        this.setOverlay("There is nothing to withdraw.", 60);
        return null;
      }
      this.mode = PCMode.BROWSE;
      this.clearPendingEntry();
      this.jumptableState = BillsPCState.END_LOOP;
      return ["withdraw", entry.boxIndex, entry.slotIndex];
    }
    if (label === "release") {
      this.mode = PCMode.BROWSE;
      this.clearPendingEntry();
      this.jumptableState = BillsPCState.END_LOOP;
      return ["release", entry.boxIndex, entry.slotIndex];
    }
    this.mode = PCMode.BROWSE;
    this.clearPendingEntry();
    return null;
  }

  private enterActionMode(entry: BillsPCEntry): void {
    this.pendingEntry = entry;
    this.actionLabels = [...this.actionLabelsForEntry()];
    this.actionNav.updateActionCount(this.actionLabels.length);
    this.actionIndex = 0;
    this.mode = PCMode.BROWSE;
    this.playConfirm();
    this.jumptableState = BillsPCState.WHATS_UP;
  }

  private actionLabelsForEntry(): readonly string[] {
    if (this.billAction === "deposit") {
      return DEPOSIT_ACTIONS;
    }
    if (this.billAction === "move") {
      return MOVE_ACTIONS;
    }
    return WITHDRAW_ACTIONS;
  }

  private clearPendingEntry(): void {
    this.pendingEntry = null;
    this.actionLabels = [...this.actionLabelsForEntry()];
    this.actionNav.updateActionCount(this.actionLabels.length);
    this.actionIndex = 0;
  }

  private showStatsScreen(): void {
    if (this.statsModalActive) {
      return;
    }
    const entry = this.pendingEntry;
    const pokemon = entry?.pokemon ?? null;
    if (!pokemon) {
      this.setOverlay("Stats unavailable.", 60);
      return;
    }
    this.statsScreen.showPokemon(pokemon);
    this.statsModalActive = true;
  }

  private closeStatsScreen(): void {
    this.statsScreen.reset();
    this.statsModalActive = false;
  }

  private handleMoveMode(event: KeyEvent, keyName: string, keyCode: number | null): [string, number | null, number] | null {
    this.ensureMonList();
    let moved = false;
    if (this.upKeys.has(keyName)) {
      moved = this.pressUp();
    } else if (this.downKeys.has(keyName)) {
      moved = this.pressDown();
    } else if (this.leftKeys.has(keyName)) {
      moved = this.pressLeft();
    } else if (this.rightKeys.has(keyName)) {
      moved = this.pressRight();
    }
    if (moved) {
      this.playCursor();
      return null;
    }
    const moveAction =
      this.moveNav.handleKey(keyName, keyCode) ??
      (isConfirmEvent(event) ? "confirm" : isCancelEvent(event) ? "cancel" : null);
    if (moveAction === "confirm") {
      const entry = this.currentSelectionEntry();
      if (this.moveOrigin === null || !entry || !this.moveOriginEntry) {
        this.exitMoveMode();
        return null;
      }
      const sourceBox = this.moveOriginEntry.boxIndex;
      const targetBox = entry.boxIndex;
      const payload: PCMenuActionPayload = {
        action: "move",
        box: sourceBox,
        slot: this.moveOriginEntry.slotIndex,
        target_box: targetBox,
        target_slot: entry.slotIndex,
      };
      this.pendingMovePayload = payload;
      this.exitMoveMode(true);
      this.playConfirm();
      this.jumptableState = BillsPCState.END_LOOP;
      return ["move", sourceBox, entry.slotIndex];
    }
    if (moveAction === "cancel") {
      this.exitMoveMode();
      this.jumptableState = BillsPCState.HANDLE_JOYPAD;
    }
    return null;
  }

  private beginMoveSelection(): void {
    if (this.selectionIsCancel()) {
      this.playCursor();
      return;
    }
    const entry = this.currentSelectionEntry();
    if (!entry || !entry.pokemon) {
      this.setOverlay(this.resolveOverlayText("PCString_TheresNoRoom", "There's no room!"), 60);
      this.playCursor();
      return;
    }
    if (this.pokemonHasMail(entry.pokemon)) {
      this.setOverlay(this.resolveOverlayText("PCString_RemoveMail", "Remove MAIL."), 60);
      this.playCursor();
      return;
    }
    this.moveOrigin = entry.slotIndex;
    this.moveOriginEntry = entry;
    this.pendingMovePayload = null;
    this.mode = PCMode.MOVE;
    this.playConfirm();
  }

  private exitMoveMode(preservePayload = false): void {
    this.moveOrigin = null;
    this.moveOriginEntry = null;
    if (!preservePayload) {
      this.pendingMovePayload = null;
    }
    this.mode = PCMode.BROWSE;
  }

  private handleDepositMode(event: KeyEvent, keyName: string, keyCode: number | null): [string, number | null, number] | null {
    const action =
      this.depositNav.handleKey(keyName, keyCode) ??
      (isConfirmEvent(event) ? "confirm" : isCancelEvent(event) ? "cancel" : null);
    if (!action) {
      return null;
    }
    if (action === "confirm") {
      const partySlot = this.depositNav.cursor;
      if (partySlot < 0 || partySlot >= this.party.pokemon.length) {
        this.playCursor();
        return null;
      }
      const member = this.party.pokemon[partySlot];
      if (!member) {
        this.playCursor();
        return null;
      }
      this.mode = PCMode.BROWSE;
      this.playConfirm();
      this.clearPendingEntry();
      this.jumptableState = BillsPCState.END_LOOP;
      return ["deposit", this.boxIndex, partySlot];
    }
    this.mode = PCMode.BROWSE;
    this.playCursor();
    this.clearPendingEntry();
    this.jumptableState = BillsPCState.HANDLE_JOYPAD;
    return null;
  }

  private startDepositMode(): boolean {
    if (this.partyCount() <= 1) {
      this.setOverlay(
        this.resolveOverlayText("PCString_ItsYourLastPKMN", "It's your last <PK><MN>!"),
        60
      );
      this.playCursor();
      return false;
    }
    const party = this.party.pokemon;
    this.depositNav.updatePartySize(party.length);
    for (let index = 0; index < party.length; index++) {
      if (party[index]) {
        this.depositNav.cursor = index;
        this.mode = PCMode.DEPOSIT;
        this.loadedBoxValue = 0;
        this.resetCursor();
        this.ensureMonList();
        return true;
      }
    }
    this.playCursor();
    return false;
  }

  private ensureBox(index: number): Box {
    const boxes = this.boxes;
    if (!boxes.length) {
      for (let count = 0; count < MAX_PC_BOXES; count++) {
        boxes.push(this.createCanonicalBox(count));
      }
    }
    while (boxes.length <= index) {
      boxes.push(this.createCanonicalBox(boxes.length));
    }
    return this.normalizeBox(index);
  }

  private createCanonicalBox(index: number): Box {
    return BoxSchema.parse({ name: formatDefaultBoxName(index) });
  }

  private normalizeBox(index: number): Box {
    const boxes = this.boxes;
    const current = boxes[index] ?? {};
    const name = current?.name && current.name.trim() ? current.name : formatDefaultBoxName(index);
    try {
      const normalized = BoxSchema.parse({ ...current, name });
      boxes[index] = normalized;
      return normalized;
    } catch (err) {
      throw new Error(
        `Invalid PC box at index ${index}: ${err instanceof Error ? err.message : String(err)}`
      );
    }
  }

  private ensureMonList(): void {
    const entries = this.buildMonEntries();
    this.monEntries = entries;
    this.numMonsInBox = Math.max(1, entries.length + 1);
    this.clampCursorScroll();
    this.syncWramState();
  }

  private buildMonEntries(): BillsPCEntry[] {
    if (!this.boxes.length) {
      return [];
    }
    const source = this.loadedBoxValue;
    if (source === 0) {
      return this.buildPartyEntries(source);
    }
    const boxIndex = this.boxIndexForLoadedValue(source);
    if (boxIndex === null) {
      return [];
    }
    return this.buildBoxEntries(boxIndex, source);
  }

  private buildPartyEntries(source: number): BillsPCEntry[] {
    const entries: BillsPCEntry[] = [];
    for (let slotIndex = 0; slotIndex < this.party.pokemon.length; slotIndex++) {
      const member = this.party.pokemon[slotIndex];
      if (!member) {
        break;
      }
      entries.push({
        speciesId: member.species.id ?? "",
        sourceBoxValue: source,
        boxIndex: null,
        slotIndex,
        nickname: member.nickname ?? "",
        pokemon: toPokemon(member),
      });
    }
    return entries;
  }

  private buildBoxEntries(boxIndex: number, source: number): BillsPCEntry[] {
    const entries: BillsPCEntry[] = [];
    const box = this.ensureBox(boxIndex);
    for (let slotIndex = 0; slotIndex < box.pokemon.length; slotIndex++) {
      const member = box.pokemon[slotIndex];
      if (!member) {
        break;
      }
      entries.push({
        speciesId: member.species.id ?? "",
        sourceBoxValue: source,
        boxIndex,
        slotIndex,
        nickname: member.nickname ?? "",
        pokemon: toPokemon(member),
      });
    }
    return entries;
  }

  private clampCursorScroll(): void {
    const maxScroll = Math.max(0, this.numMonsInBox - this.numMonsOnScreen);
    this.scrollPosition = Math.max(0, Math.min(this.scrollPosition, maxScroll));
    const maxCursor = Math.max(0, this.numMonsOnScreen - 1);
    this.cursorPosition = Math.max(0, Math.min(this.cursorPosition, maxCursor));
  }

  private selectedEntryIndex(): number {
    return this.scrollPosition + this.cursorPosition;
  }

  private selectionIsCancel(): boolean {
    return this.selectedEntryIndex() === this.monEntries.length;
  }

  private currentSelectionEntry(): BillsPCEntry | null {
    const index = this.selectedEntryIndex();
    if (index < this.monEntries.length) {
      return this.monEntries[index];
    }
    return null;
  }

  private pressUp(): boolean {
    let moved = false;
    if (this.cursorPosition > 0) {
      this.cursorPosition -= 1;
      moved = true;
    } else if (this.scrollPosition > 0) {
      this.scrollPosition -= 1;
      moved = true;
    }
    if (moved) {
      this.syncWramState();
    }
    return moved;
  }

  private pressDown(): boolean {
    if (this.selectedEntryIndex() + 1 >= this.numMonsInBox) {
      return false;
    }
    let moved = false;
    if (this.cursorPosition + 1 < this.numMonsOnScreen) {
      this.cursorPosition += 1;
      moved = true;
    } else {
      this.scrollPosition += 1;
      const maxScroll = Math.max(0, this.numMonsInBox - this.numMonsOnScreen);
      if (this.scrollPosition > maxScroll) {
        this.scrollPosition = maxScroll;
      }
      moved = true;
    }
    if (moved) {
      this.syncWramState();
    }
    return true;
  }

  private pressLeft(): boolean {
    if (this.billAction !== "move") {
      return false;
    }
    const previous = this.loadedBoxValue;
    const target = this.normalizeLoadedBoxValue(previous - 1);
    if (target === previous) {
      return false;
    }
    this.loadedBoxValue = target;
    this.resetCursor();
    this.syncBoxIndex();
    this.ensureMonList();
    return true;
  }

  private pressRight(): boolean {
    if (this.billAction !== "move") {
      return false;
    }
    const previous = this.loadedBoxValue;
    const target = this.normalizeLoadedBoxValue(previous + 1);
    if (target === previous) {
      return false;
    }
    this.loadedBoxValue = target;
    this.resetCursor();
    this.syncBoxIndex();
    this.ensureMonList();
    return true;
  }

  private loadedStateCount(): number {
    return this.boxes.length + 1;
  }

  private normalizeLoadedBoxValue(value: number): number {
    if (this.billAction === "deposit") {
      return 0;
    }
    if (this.billAction === "withdraw") {
      if (!this.boxes.length) {
        return BILLS_PC_CURRENT_BOX_VALUE;
      }
      if (value === BILLS_PC_CURRENT_BOX_VALUE) {
        return BILLS_PC_CURRENT_BOX_VALUE;
      }
      const count = this.boxes.length;
      const zeroBased = ((value - 1) % count + count) % count;
      return zeroBased + 1;
    }
    const count = this.loadedStateCount();
    if (count <= 0) {
      return 0;
    }
    const mod = value % count;
    return mod < 0 ? mod + count : mod;
  }

  private resetCursor(): void {
    this.cursorPosition = 0;
    this.scrollPosition = 0;
    this.syncWramState();
  }

  private syncBoxIndex(): void {
    const boxIndex = this.boxIndexForLoadedValue(this.loadedBoxValue);
    if (boxIndex !== null) {
      this.boxIndex = boxIndex;
      this.setCurrentPcBox(boxIndex);
    }
  }

  private boxIndexForLoadedValue(value: number): number | null {
    if (value === 0) {
      return null;
    }
    if (!this.boxes.length) {
      return null;
    }
    if (value === BILLS_PC_CURRENT_BOX_VALUE) {
      return Math.max(0, Math.min(this.boxIndex, this.boxes.length - 1));
    }
    const candidate = value - 1;
    return Math.max(0, Math.min(candidate, this.boxes.length - 1));
  }

  private syncWramState(): void {
    const wram = this.gameState.wram;
    wram.wBillsPC_LoadedBox = this.loadedBoxValue;
    wram.wBillsPC_CursorPosition = this.cursorPosition;
    wram.wBillsPC_ScrollPosition = this.scrollPosition;
    wram.wBillsPC_NumMonsOnScreen = this.numMonsOnScreen;
    wram.wBillsPC_NumMonsInBox = this.numMonsInBox;
  }

  private currentListWindow(): { scroll: number; cursor: number } {
    if (this.mode !== PCMode.DEPOSIT || this.depositNav.cursor < 0) {
      return { scroll: this.scrollPosition, cursor: this.cursorPosition };
    }
    const maxScroll = Math.max(0, this.monEntries.length + 1 - BILLS_PC_ROWS);
    const scroll = Math.max(0, Math.min(this.depositNav.cursor - (BILLS_PC_ROWS - 1), maxScroll));
    const cursor = Math.max(0, this.depositNav.cursor - scroll);
    return { scroll, cursor };
  }

  draw(): void {
    const screen = this.ui.screen;
    if (!screen) {
      return;
    }
    screen.fill([255, 255, 255, 255]);
    const { tilemap, iconIds } = createPcTilemap();
    this.depositNav.updatePartySize(this.party.pokemon.length);
    this.ensureMonList();
    const listWindow = this.currentListWindow();
    const depositCursor =
      this.mode === PCMode.DEPOSIT && this.depositNav.partySize > 0
        ? this.depositNav.cursor
        : null;
    const selected = this.resolveSelectedPokemon(depositCursor);
    this.listView.draw(this.ui, tilemap, this.monEntries, listWindow.scroll, listWindow.cursor);
    if (this.mode === PCMode.ACTIONS) {
      this.actionsView.draw(
        this.ui,
        tilemap,
        this.actionLabels,
        this.actionIndex,
        this.party.pokemon.some(Boolean),
        this.mode
      );
    }
    this.drawInfoPanel(tilemap, selected, iconIds);
    const currentBox = this.currentBox();
    const showingParty = this.loadedBoxValue === 0 && this.mode === PCMode.MOVE;
    const boxLabel = showingParty ? `PARTY ${POKEMON_WORD}` : currentBox.name || formatDefaultBoxName(this.boxIndex);
    this.messageView.draw(this.ui, tilemap, this.boxIndex, boxLabel, {
      showArrows: this.billAction === "move",
    });
    const prompt = this.resolvePromptText();
    this.bottomPromptView.draw(this.ui, tilemap, prompt);
    this.renderTextSnapshot(selected, boxLabel, listWindow, prompt);
    blitPcTilemap(screen, this.ui.font, tilemap);
    this.drawSelectedMonPreview(selected);
    if (this.mode !== PCMode.ACTIONS) {
      this.cursorView.draw(screen, listWindow.cursor, this.mode === PCMode.MOVE ? "insert" : "selection");
    }
    this.drawOverlay();
  }

  private statusText(): string {
    if (this.mode === PCMode.MOVE) {
      return "MOVE TO WHERE?";
    }
    if (this.mode === PCMode.DEPOSIT || this.billAction === "deposit") {
      return "SELECT A POK\u00e9MON.";
    }
    if (this.mode === PCMode.ACTIONS) {
      return "WHAT'S UP?";
    }
    return "CHOOSE A POK\u00e9MON.";
  }

  private resolvePromptText(): string {
    const label = PokemonPCMenu.PROMPT_LABELS[this.mode] ?? "PCString_ChooseaPKMN";
    const fallback = this.statusText();
    if (!label || typeof this.dataLoader?.getText !== "function") {
      return fallback;
    }
    try {
      const text = this.dataLoader.getText(label);
      if (typeof text === "string" && text.trim()) {
        return text.trim();
      }
    } catch {
      return fallback;
    }
    return fallback;
  }

  private drawInfoPanel(
    tilemap: TilemapSurface,
    pokemon: Pokemon | null,
    iconIds: Record<string, number>
  ): void {
    tilemap.fillRect(
      INFO_CLEAR_REGION.x,
      INFO_CLEAR_REGION.y,
      INFO_CLEAR_REGION.width,
      INFO_CLEAR_REGION.height,
      { tile: SPACE_TILE, attr: PC_TEXT_PALETTE }
    );
    tilemap.fillRect(8, 14, 3, 1, { tile: SPACE_TILE, attr: PC_TEXT_PALETTE });
    if (!pokemon) {
      return;
    }
    const name = this.localizedName(pokemon).slice(0, INFO_NAME_MAX_CHARS);
    tilemap.writeText(INFO_NAME_ORIGIN.x, INFO_NAME_ORIGIN.y, name, {
      maxLength: INFO_NAME_MAX_CHARS,
      pad: true,
      uppercase: true,
    });
    const levelText = `${LV_GLYPH}${pokemon.level}`;
    tilemap.writeText(INFO_LEVEL_ORIGIN.x, INFO_LEVEL_ORIGIN.y, levelText, {
      maxLength: 4,
      pad: false,
      uppercase: true,
    });
    const genderSymbol = this.genderSymbol(pokemon.gender as PlayerGender | undefined);
    if (genderSymbol) {
      tilemap.writeText(INFO_GENDER_ORIGIN.x, INFO_GENDER_ORIGIN.y, genderSymbol, {
        maxLength: 1,
        pad: false,
      });
    }
    this.writeItemIcon(tilemap, pokemon, iconIds);
  }

  private renderTextSnapshot(
    selected: Pokemon | null,
    boxLabel: string,
    listWindow: { scroll: number; cursor: number },
    prompt: string
  ): void {
    if (!this.ui.renderSnapshot) {
      return;
    }
    const title = this.billAction === "deposit"
      ? `DEPOSIT ${POKEMON_WORD}`
      : this.billAction === "move"
        ? `MOVE ${POKEMON_WORD} W/O MAIL`
        : `WITHDRAW ${POKEMON_WORD}`;
    const viewportLines = [title, boxLabel];
    for (let row = 0; row < BILLS_PC_ROWS; row += 1) {
      const absoluteIndex = listWindow.scroll + row;
      const cursor = row === listWindow.cursor ? "▶" : " ";
      viewportLines.push(`${cursor} ${this.listLabelForIndex(absoluteIndex)}`);
    }

    const infoLines = selected
      ? [
          `SELECTED: ${this.localizedName(selected).toUpperCase()}`,
          `LEVEL: ${selected.level}`,
          `GENDER: ${this.genderSymbol(selected.gender as PlayerGender | undefined) ?? "-"}`,
          `ITEM: ${selected.item ? this.formatItemName(selected.item) : "-"}`,
        ]
      : ["SELECTED: CANCEL"];
    infoLines.push("D-Pad=Move A=Select B=Back");

    const menuLines =
      this.mode === PCMode.ACTIONS
        ? this.actionLabels.map((label, index) => `${index === this.actionIndex ? "▶" : " "} ${label}`)
        : null;
    this.ui.renderSnapshot(viewportLines, infoLines, "Bill's PC", "Legend", menuLines, [prompt], null);
  }

  private listLabelForIndex(index: number): string {
    if (index < this.monEntries.length) {
      const nickname = this.monEntries[index].nickname || BILLS_PC_PLACEHOLDER;
      return nickname.toUpperCase().slice(0, BILLS_PC_LIST_NAME_MAX_CHARS);
    }
    if (index === this.monEntries.length) {
      return BILLS_PC_CANCEL_LABEL;
    }
    return "";
  }

  private resolveSelectedPokemon(depositCursor: number | null): Pokemon | null {
    if (this.mode === PCMode.MOVE && this.moveOriginEntry?.pokemon) {
      return this.moveOriginEntry.pokemon;
    }
    if (this.mode === PCMode.ACTIONS && this.pendingEntry?.pokemon) {
      return this.pendingEntry.pokemon;
    }
    if (this.mode === PCMode.DEPOSIT && depositCursor !== null) {
      if (depositCursor >= 0 && depositCursor < this.party.pokemon.length) {
        const mon = this.party.pokemon[depositCursor];
        return mon ? toPokemon(mon) : null;
      }
      return null;
    }
    const entry = this.currentSelectionEntry();
    return entry?.pokemon ?? null;
  }

  private localizedName(pokemon: Pokemon): string {
    const nickname = (pokemon.nickname ?? "").trim();
    if (nickname) {
      return nickname;
    }
    return pokemon.species.id ?? BILLS_PC_PLACEHOLDER;
  }

  private genderSymbol(gender?: PlayerGender | null): string | null {
    if (gender === PlayerGender.MALE) {
      return "♂";
    }
    if (gender === PlayerGender.FEMALE) {
      return "♀";
    }
    return null;
  }

  private writeItemIcon(tilemap: TilemapSurface, pokemon: Pokemon, iconIds: Record<string, number>): void {
    const itemName = pokemon.item ?? "";
    if (!itemName) {
      return;
    }
    const tileId = isMailItem(itemName) ? iconIds.mail : iconIds.item;
    if (!tileId) {
      return;
    }
    tilemap.setTile(INFO_ITEM_ORIGIN.x, INFO_ITEM_ORIGIN.y, tileId);
  }

  private pokemonHasMail(pokemon: Pokemon | null): boolean {
    if (!pokemon) {
      return false;
    }
    return isMailItem(pokemon.item ?? null);
  }

  private anyPokemonHoldingMail(): boolean {
    for (const member of this.party.pokemon) {
      if (member && this.pokemonHasMail(toPokemon(member))) {
        return true;
      }
    }
    for (const box of this.boxes) {
      for (const member of box.pokemon) {
        if (member && this.pokemonHasMail(toPokemon(member))) {
          return true;
        }
      }
    }
    return false;
  }

  private drawSelectedMonPreview(pokemon: Pokemon | null): void {
    if (!pokemon) {
      return;
    }
    this.ui.drawSprite(
      pokemon.species.id,
      INFO_PIC_ORIGIN.x * TILE_SIZE,
      INFO_PIC_ORIGIN.y * TILE_SIZE,
      "pokemon",
      0
    );
  }

  private drawOverlay(): void {
    const screen = this.ui.screen;
    if (!screen || !this.overlayMessage) {
      return;
    }
    if (this.overlayFrames <= 0) {
      this.overlayMessage = null;
      return;
    }
    const rect = new Rect(TILE_SIZE * 6, TILE_SIZE * 7, TILE_SIZE * 8, TILE_SIZE * 4);
    this.ui.drawWindow(screen, rect.x, rect.y, rect.width / TILE_SIZE, rect.height / TILE_SIZE, {
      fill: PC_WINDOW_FILL,
    });
    renderFontText(this.ui.font, this.overlayMessage, rect.x + TILE_SIZE, rect.y + TILE_SIZE, screen);
    this.overlayFrames -= 1;
  }

  setOverlay(message: string, duration = 30): void {
    this.overlayMessage = message;
    this.overlayFrames = duration;
  }

  private advanceJumptableState(): void {
    if (this.statsModalActive) {
      return;
    }
    if (this.jumptableState === BillsPCState.INIT) {
      this.runInitState();
    } else if (this.jumptableState === BillsPCState.WHATS_UP) {
      this.runWhatsUpState();
    } else if (this.jumptableState === BillsPCState.END_LOOP) {
      this.runEndLoopState();
    }
  }

  private runInitState(): void {
    this.ensureMonList();
    this.mode = this.billAction === "deposit" ? PCMode.DEPOSIT : PCMode.BROWSE;
    this.jumptableState = BillsPCState.HANDLE_JOYPAD;
  }

  private runWhatsUpState(): void {
    this.mode = PCMode.ACTIONS;
    this.jumptableState = BillsPCState.SUBMENU;
  }

  private runEndLoopState(): void {
    if (this.mode === PCMode.ACTIONS) {
      this.mode = PCMode.BROWSE;
    }
    this.jumptableState = BillsPCState.HANDLE_JOYPAD;
  }

  private partyCount(): number {
    return this.party.pokemon.filter((pokemon) => pokemon !== null).length;
  }

  private resolveOverlayText(label: string, fallback: string): string {
    let text = fallback;
    if (typeof this.dataLoader?.getText === "function") {
      try {
        const loaded = this.dataLoader.getText(label);
        if (loaded) {
          text = loaded;
        }
      } catch {
        // Keep fallback text.
      }
    }
    return this.formatOverlayText(text);
  }

  private resolveMovePromptText(): string {
    const defaultText = this.formatOverlayText(
      `Each time you move a ${POKEMON_WORD}, data will be saved. OK?`
    );
    if (typeof this.dataLoader?.getText !== "function") {
      return defaultText;
    }
    try {
      const text = this.dataLoader.getText("MoveMonWOMailSaveText") ?? "";
      if (typeof text !== "string" || !text.trim()) {
        return defaultText;
      }
      return this.formatOverlayText(text);
    } catch {
      return defaultText;
    }
  }

  private promptMoveWithoutMail(drawCallback?: () => void): boolean {
    const text = this.resolveMovePromptText();
    if (!isPromptUI(this.ui)) {
      return true;
    }
    const prompt = new SelectionPrompt(this.ui, ["YES", "NO"], {
      audioEngine: this.audioEngine ?? undefined,
      title: text,
      windowOriginTiles: [0, 1],
      windowMinWidth: 18,
    });
    const selection = prompt.run({ drawCallback: this.composePromptDrawCallback(drawCallback) });
    if (selection !== 0) {
      return false;
    }
    void saveGame(this.gameState, "savegame");
    return true;
  }

  private async promptMoveWithoutMailAsync(drawCallback?: () => void): Promise<boolean> {
    const text = this.resolveMovePromptText();
    if (!isPromptUI(this.ui)) {
      return true;
    }
    const prompt = new SelectionPrompt(this.ui, ["YES", "NO"], {
      audioEngine: this.audioEngine ?? undefined,
      title: text,
      windowOriginTiles: [0, 1],
      windowMinWidth: 18,
    });
    const selection = await prompt.runAsync({ drawCallback: this.composePromptDrawCallback(drawCallback) });
    if (selection !== 0) {
      return false;
    }
    void saveGame(this.gameState, "savegame");
    return true;
  }

  async startMoveSessionAsync(
    {
      confirmationProvider,
      drawCallback,
    }: { confirmationProvider?: () => Promise<boolean>; drawCallback?: () => void } = {}
  ): Promise<boolean> {
    if (this.anyPokemonHoldingMail()) {
      this.setOverlay(this.resolveOverlayText("PCString_RemoveMail", "Remove MAIL."), 60);
      return false;
    }
    const provider = confirmationProvider ?? (() => this.promptMoveWithoutMailAsync(drawCallback));
    if (!(await provider())) {
      this.stopMoveSession();
      return false;
    }
    this.moveSessionActive = true;
    this.moveOrigin = null;
    this.moveOriginEntry = null;
    this.pendingMovePayload = null;
    return true;
  }

  private formatOverlayText(text: string): string {
    return text
      .split("<PK><MN>")
      .join(POKEMON_WORD)
      .split("#MON")
      .join(POKEMON_WORD)
      .split("<PLAYER>")
      .join(this.playerName());
  }

  private formatItemName(itemId: unknown): string {
    return String(itemId || "ITEM").replace(/_/g, " ").trim().toUpperCase();
  }

  private playerName(): string {
    const name = this.gameState.sram.player_name ?? "PLAYER";
    return name.trim() || "PLAYER";
  }

  private currentBox(): Box {
    const total = Math.max(this.boxes.length, 1);
    return this.ensureBox(this.boxIndex % total);
  }

  runInteractive(
    {
      actionHandler,
      drawCallback,
    }: {
      actionHandler?: (payload: PCMenuActionPayload) => PCMenuInteractiveResponse | null;
      drawCallback?: () => void;
    } = {}
  ): PCMenuInteractiveResponse[] {
    const screen = this.ui.screen;
    if (!screen) {
      return [];
    }
    const results: PCMenuInteractiveResponse[] = [];
    let running = true;
    this.statsModalActive = false;
    this.statsScreen.reset();
    this.mode = PCMode.BROWSE;
    this.jumptableState = BillsPCState.INIT;
    while (running) {
      this.advanceJumptableState();
      const events = this.ui.pollEvents ? this.ui.pollEvents() : [];
      for (const event of events) {
        if (event.type === "quit") {
          throw new Error("Quit requested while PC menu active.");
        }
        if (this.statsModalActive) {
          const result = this.statsScreen.handleInput(event);
          if (result === "exit") {
            this.closeStatsScreen();
          }
          continue;
        }
        const handled = this.handleInput(event);
        if (!handled) {
          continue;
        }
        const [action, boxIndex, slotIndex] = handled;
        if (action === "cancel") {
          running = false;
          break;
        }
        const payload: PCMenuActionPayload = { action, box: boxIndex, slot: slotIndex };
        if (action === "deposit") {
          payload.party_slot = slotIndex;
          payload.slot = null;
        }
        if (action === "move" && this.pendingMovePayload) {
          Object.assign(payload, this.pendingMovePayload);
          this.pendingMovePayload = null;
        }
        const response = actionHandler ? actionHandler(payload) : payload;
        if (response) {
          results.push(response);
        }
      }
      if (drawCallback) {
        drawCallback();
      }
      if (this.statsModalActive) {
        this.statsScreen.draw();
      } else {
        this.draw();
      }
      this.ui.update();
    }
    return results;
  }

  async runInteractiveAsync(
    {
      actionHandler,
      drawCallback,
    }: {
      actionHandler?: (payload: PCMenuActionPayload) => PCMenuInteractiveResponse | Promise<PCMenuInteractiveResponse | null> | null;
      drawCallback?: () => void;
    } = {}
  ): Promise<PCMenuInteractiveResponse[]> {
    const screen = this.ui.screen;
    if (!screen) {
      return [];
    }
    const results: PCMenuInteractiveResponse[] = [];
    let running = true;
    this.statsModalActive = false;
    this.statsScreen.reset();
    this.mode = PCMode.BROWSE;
    this.jumptableState = BillsPCState.INIT;
    while (running) {
      this.advanceJumptableState();
      const events = this.ui.pollEvents ? this.ui.pollEvents() : [];
      for (const event of events) {
        if (event.type === "quit") {
          throw new Error("Quit requested while PC menu active.");
        }
        if (this.statsModalActive) {
          const result = this.statsScreen.handleInput(event);
          if (result === "exit") {
            this.closeStatsScreen();
          }
          continue;
        }
        const handled = this.handleInput(event);
        if (!handled) {
          continue;
        }
        const [action, boxIndex, slotIndex] = handled;
        if (action === "cancel") {
          running = false;
          break;
        }
        const payload: PCMenuActionPayload = { action, box: boxIndex, slot: slotIndex };
        if (action === "deposit") {
          payload.party_slot = slotIndex;
          payload.slot = null;
        }
        if (action === "move" && this.pendingMovePayload) {
          Object.assign(payload, this.pendingMovePayload);
          this.pendingMovePayload = null;
        }
        const response = actionHandler ? await actionHandler(payload) : payload;
        if (response) {
          results.push(response);
        }
      }
      if (drawCallback) {
        drawCallback();
      }
      if (this.statsModalActive) {
        this.statsScreen.draw();
      } else {
        this.draw();
      }
      this.ui.update();
      await nextFrame();
    }
    return results;
  }

  private playCursor(): void {
    this.audioEngine?.playSound("menu_cursor");
  }

  private playConfirm(): void {
    this.audioEngine?.playSound("menu_option");
  }

  private playSwitchSound(): void {
    playPcSwitchSound(this.audioEngine ?? null);
  }

  private composePromptDrawCallback(drawCallback?: () => void): () => void {
    return () => {
      if (drawCallback) {
        drawCallback();
      }
      this.draw();
    };
  }
}
