// ASM mapping: pokecrystal_disassembly/engine/menus/party_menu.asm (party menu behavior + input).
import { gameEngine, GameEngineEvent } from "../game-engine";
import { B_PAD_A, B_PAD_B, B_PAD_DOWN, B_PAD_UP, isKeyDownEvent, isKeyUpEvent } from "@pokecrystal/core/input/controls";
import { updateJoypadStateFromKeys } from "@pokecrystal/core/core/joypad";
import { Pokemon, toPokemon } from "../../core/models";
import { MoveName } from "../../core/enums/move";
import { MonType } from "../../core/enums/pokemon";
import { MonMenuItem, MON_MENU_OPTION_STRINGS } from "../../core/enums/mon-menu";
import { PartyMenuAction, PARTY_MENU_PROMPTS, PARTY_MENU_QUALITY_POINTERS } from "../../core/enums/party-menu";
import { GameState } from "../../core/state";
import { AudioEngine } from "../../engine/systems/audio";
import { ItemSystem } from "../../engine/systems/items";
import { BGMapWriter } from "../bg-map-sync";
import { TileRegion } from "../tile-layout";
import { Surface } from "../surface";
import type { BaseFontRenderer } from "../base-ui";
import type { Palette } from "../font-renderer";
import {
  PartyEntry,
  PartyMenuTilemap,
  partyMenuTileset,
  FIELD_MOVE_LOOKUP,
  GIVE_TAKE_REGION,
  MENU_OPTION_LOOKUP,
  NO_PARTY_TEXT,
  PAL_BG_TEXT,
  POINTER_COLUMN,
  SUBMENU_REGION,
  TEXTBOX_BORDER_REGION,
  TEXTBOX_REGION,
  NUM_MON_SUBMENU_ITEMS,
  partyMenuCancelRow,
  partyMenuNameRow,
  partyMenuStatusRow,
  PARTY_LIST_REGION,
  DETAIL_REGION,
  HP_BAR_LENGTH_PX,
} from "./party-menu-layout";
import { PartyMenuIconRenderer } from "./party-menu-icons";
import { PartyMenuQualityRenderer } from "./party-menu-qualities";

type PartyMenuChoice = {
  item: MonMenuItem;
  label: string;
  moveEntry: (typeof FIELD_MOVE_LOOKUP)[string] | null;
};

type SelectionResult = [string, number];

export type PokemonMenuUI = {
  screen: Surface | null;
  font: BaseFontRenderer & {
    paletteVariants?: (paletteOrder: ReadonlyArray<Palette>) => Record<number, Record<number, Surface>>;
    fontTiles?: Record<number, Surface>;
    font_tiles?: Record<number, Surface>;
  };
};

const HP_ANIM_STEP_FRAMES = 2;

class PartyHpAnimationState {
  currentPixels = 0;
  targetPixels = 0;
  currentHp = 0;
  targetHp = 0;
  maxHp = 0;
  framesUntilStep = 0;
  direction = 0;
  active = false;
  initialized = false;

  start(fromHp: number, toHp: number, maxHp: number): void {
    const clampedMax = Math.max(0, maxHp);
    const startHp = Math.max(0, Math.min(fromHp, clampedMax));
    const endHp = Math.max(0, Math.min(toHp, clampedMax));
    const startPixels = PartyHpAnimationState.hpToPixels(startHp, clampedMax);
    const endPixels = PartyHpAnimationState.hpToPixels(endHp, clampedMax);
    this.currentHp = startHp;
    this.targetHp = endHp;
    this.currentPixels = startPixels;
    this.targetPixels = endPixels;
    this.maxHp = clampedMax;
    if (startPixels === endPixels) {
      this.direction = 0;
      this.active = false;
      this.framesUntilStep = 0;
      this.initialized = true;
      return;
    }
    this.direction = endPixels > startPixels ? 1 : -1;
    this.active = true;
    this.framesUntilStep = 0;
    this.initialized = true;
  }

  step(): number {
    if (!this.active) {
      return this.currentHp;
    }
    if (this.framesUntilStep > 0) {
      this.framesUntilStep -= 1;
      return this.currentHp;
    }
    this.framesUntilStep = HP_ANIM_STEP_FRAMES;
    this.currentPixels = Math.max(
      0,
      Math.min(HP_BAR_LENGTH_PX, this.currentPixels + this.direction),
    );
    if (
      (this.direction > 0 && this.currentPixels >= this.targetPixels) ||
      (this.direction < 0 && this.currentPixels <= this.targetPixels)
    ) {
      this.currentPixels = this.targetPixels;
      this.currentHp = this.targetHp;
      this.active = false;
      return this.currentHp;
    }
    this.currentHp = PartyHpAnimationState.pixelsToHp(this.currentPixels, this.maxHp);
    this.currentHp = PartyHpAnimationState.clampHp(this.currentHp, this.targetHp, this.direction);
    return this.currentHp;
  }

  private static clampHp(value: number, target: number, direction: number): number {
    if (direction > 0 && value > target) {
      return target;
    }
    if (direction < 0 && value < target) {
      return target;
    }
    return value;
  }

  private static hpToPixels(hp: number, maxHp: number): number {
    if (maxHp <= 0 || hp <= 0) {
      return 0;
    }
    let pixels = Math.floor((hp * HP_BAR_LENGTH_PX) / maxHp);
    if (pixels === 0) {
      pixels = 1;
    }
    return Math.min(HP_BAR_LENGTH_PX, pixels);
  }

  private static pixelsToHp(pixels: number, maxHp: number): number {
    if (maxHp <= 0 || pixels <= 0) {
      return 0;
    }
    const rounded = Math.floor((pixels * maxHp + HP_BAR_LENGTH_PX - 1) / HP_BAR_LENGTH_PX);
    return Math.max(1, Math.min(maxHp, rounded));
  }
}

const ACTION_SET = new Set(
  Object.values(PartyMenuAction).filter((value): value is number => typeof value === "number"),
);

const actionFromWram = (value: number): PartyMenuAction => {
  if (ACTION_SET.has(value)) {
    return value as PartyMenuAction;
  }
  return PartyMenuAction.CHOOSE_POKEMON;
};

const normalizeKey = (event: GameEngineEvent): string | null => {
  if (event.code) {
    return String(event.code);
  }
  if (event.key) {
    return String(event.key);
  }
  return null;
};

export class PokemonMenu {
  private readonly screen;
  private readonly party: GameState["sram"]["party"];
  private readonly itemSystem: ItemSystem;
  private readonly iconRenderer = new PartyMenuIconRenderer();
  private readonly qualityRenderer = new PartyMenuQualityRenderer();
  private readonly tileset;
  private readonly bgMapWriter: BGMapWriter;
  private cursorIndex = 0;
  private mode: "list" | "submenu" | "switch" | "give_take" = "list";
  private action: PartyMenuAction;
  private battleMenu: boolean;
  private switchBehavior: "swap" | "select";
  private switchOrigin: number | null = null;
  private submenuChoices: PartyMenuChoice[] = [];
  private submenuIndex = 0;
  private submenuEntry: PartyEntry | null = null;
  private submenuRowStart = SUBMENU_REGION.top + 1;
  private itemMenuEntry: PartyEntry | null = null;
  private giveTakeIndex = 0;
  private selectionHandler: ((pokemon: Pokemon, index?: number) => boolean | void) | null = null;
  private selectionCancel: (() => void) | null = null;
  _tmhm_move: MoveName | null = null;
  _current_item: string | null = null;
  private iconsFrozen = false;
  private hpAnimations: Map<number, PartyHpAnimationState> = new Map();
  private switchModeAction: PartyMenuAction | null = null;

  private defaultAction: PartyMenuAction;
  private itemSelector: ((pokemon: Pokemon) => string | null | Promise<string | null>) | null = null;
  private itemSelectionInProgress = false;
  private actionHandlers = new Map<MonMenuItem, (menu: PokemonMenu, pokemon: Pokemon, index?: number) => void>();

  constructor(
    private readonly ui: PokemonMenuUI,
    public readonly gameState: GameState,
    private readonly audioEngine?: AudioEngine | null,
    options?: { action?: PartyMenuAction; battle_menu?: boolean; switch_behavior?: "swap" | "select" },
  ) {
    this.screen = ui.screen;
    this.party = this.gameState.sram.party;
    this.defaultAction = options?.action ?? PartyMenuAction.CHOOSE_POKEMON;
    this.action = this.defaultAction;
    this.battleMenu = options?.battle_menu ?? false;
    this.switchBehavior = options?.switch_behavior ?? "swap";
    this.itemSystem = new ItemSystem(gameState);
    const fontSource =
      this.ui.font.paletteVariants ? this.ui.font : this.ui.font.fontTiles ?? this.ui.font.font_tiles;
    if (!fontSource) {
      throw new Error("Party menu requires font palette variants or font tiles.");
    }
    this.tileset = partyMenuTileset(fontSource);
    this.bgMapWriter = new BGMapWriter(gameState, "vBGMap0");
    this.reset();
  }

  static layoutRegions(): Record<string, TileRegion> {
    return {
      list: PARTY_LIST_REGION,
      submenu: SUBMENU_REGION,
      give_take: GIVE_TAKE_REGION,
      textbox: TEXTBOX_REGION,
      detail: DETAIL_REGION,
    };
  }

  setItemSelector(selector: (pokemon: Pokemon) => string | null | Promise<string | null>): void {
    this.itemSelector = selector;
  }

  setAction(action: PartyMenuAction): void {
    this.action = action;
    this.syncWramState();
  }

  requestSelection(opts: { handler: (pokemon: Pokemon, index?: number) => boolean | void; cancelHandler?: () => void }): void {
    this.selectionHandler = opts.handler;
    this.selectionCancel = opts.cancelHandler ?? null;
    this.mode = "list";
    this.syncWramState();
  }

  clearSelectionRequest(): void {
    this.selectionHandler = null;
    this.selectionCancel = null;
    this.mode = "list";
    this.syncWramState();
  }

  startHpAnimation(partyIndex: number, fromHp: number, toHp: number, maxHp: number): void {
    if (partyIndex < 0) {
      return;
    }
    const state = new PartyHpAnimationState();
    state.start(fromHp, toHp, maxHp);
    if (state.initialized) {
      this.hpAnimations.set(partyIndex, state);
    }
  }

  setTmhmMove(move: MoveName | null): void {
    this._tmhm_move = move;
  }

  setEvolutionItem(item: string | null): void {
    this._current_item = PartyMenuQualityRenderer.canonicalizeItem(item);
  }

  registerMonMenuHandler(item: MonMenuItem, handler: (menu: PokemonMenu, pokemon: Pokemon, index?: number) => void): void {
    this.actionHandlers.set(item, handler);
  }

  reset(options: { preserveWramCursor?: boolean } = {}): void {
    const preservedCursor = options.preserveWramCursor
      ? this.cursorIndexFromWram()
      : 0;
    this.cursorIndex = preservedCursor;
    this.mode = "list";
    this.action = this.defaultAction;
    this.switchModeAction = null;
    this.switchOrigin = null;
    this.submenuChoices = [];
    this.submenuIndex = 0;
    this.submenuEntry = null;
    this.submenuRowStart = SUBMENU_REGION.top + 1;
    this.itemMenuEntry = null;
    this.giveTakeIndex = 0;
    this.selectionHandler = null;
    this.selectionCancel = null;
    this._tmhm_move = null;
    this._current_item = null;
    this.iconsFrozen = false;
    this.hpAnimations.clear();
    this.gameState.wram.wMenuJoypadFilter = B_PAD_A | B_PAD_B;
    this.gameState.wram.menu_sprite_anims_enabled = false;
    this.gameState.wram.wMenuSelection = 0;
    this.syncWramState();
  }

  private cursorIndexFromWram(): number {
    const entries = this.partyEntries();
    if (!entries.length) {
      return 0;
    }
    const cursor = Number(this.gameState.wram.wPartyMenuCursor ?? 0);
    if (!Number.isFinite(cursor) || cursor <= 0) {
      return 0;
    }
    return Math.max(0, Math.min(cursor - 1, entries.length));
  }

  getCursorIndex(): number {
    return this.cursorIndex;
  }

  getMode(): "list" | "submenu" | "switch" | "give_take" {
    return this.mode;
  }

  getAction(): PartyMenuAction {
    return this.action;
  }

  getSwitchOrigin(): number | null {
    return this.switchOrigin;
  }

  getGiveTakeIndex(): number {
    return this.giveTakeIndex;
  }

  getSubmenuChoices(): PartyMenuChoice[] {
    return [...this.submenuChoices];
  }

  getSubmenuIndex(): number {
    return this.submenuIndex;
  }

  getPartyEntries(): PartyEntry[] {
    return this.partyEntries();
  }

  getActiveEntry(): PartyEntry | null {
    return this.submenuEntry ?? this.currentEntry();
  }

  set_item_selector(selector: (pokemon: Pokemon) => string | null | Promise<string | null>): void {
    this.setItemSelector(selector);
  }

  set_action(action: PartyMenuAction): void {
    this.setAction(action);
  }

  request_selection(opts: { handler: (pokemon: Pokemon) => boolean | void; cancel_handler?: () => void }): void {
    this.requestSelection({ handler: opts.handler, cancelHandler: opts.cancel_handler });
  }

  clear_selection_request(): void {
    this.clearSelectionRequest();
  }

  set_tmhm_move(move: MoveName | null): void {
    this.setTmhmMove(move);
  }

  set_evolution_item(item: string | null): void {
    this.setEvolutionItem(item);
  }

  register_mon_menu_handler(item: MonMenuItem, handler: (menu: PokemonMenu, pokemon: Pokemon, index?: number) => void): void {
    this.registerMonMenuHandler(item, handler);
  }

  _hp_bar_tiles(pokemon: Pokemon): [number[], number] {
    return this.qualityRenderer.hpBarTiles(pokemon);
  }

  _hp_palette_index_from_pixels(pixels: number): number {
    return PartyMenuQualityRenderer.hpPaletteIndexFromPixels(pixels);
  }

  _place_party_hp_bars(tilemap: PartyMenuTilemap, entries: PartyEntry[]): void {
    this.qualityRenderer.placePartyHpBars(this, tilemap, entries);
  }

  _place_party_hp_digits(tilemap: PartyMenuTilemap, entries: PartyEntry[]): void {
    this.qualityRenderer.placePartyHpDigits(this, tilemap, entries);
  }

  _place_party_levels(tilemap: PartyMenuTilemap, entries: PartyEntry[]): void {
    this.qualityRenderer.placePartyLevels(this, tilemap, entries);
  }

  _place_party_status(tilemap: PartyMenuTilemap, entries: PartyEntry[]): void {
    this.qualityRenderer.placePartyStatus(this, tilemap, entries);
  }

  private partyEntries(): PartyEntry[] {
    const entries: PartyEntry[] = [];
    this.party.pokemon.forEach((member, index) => {
      if (member) {
        const pokemon = toPokemon(member);
        const animation = this.hpAnimations.get(index) ?? null;
        if (animation) {
          entries.push({ index, pokemon: { ...pokemon, hp: animation.currentHp } });
        } else {
          entries.push({ index, pokemon });
        }
      }
    });
    return entries;
  }

  private highlightSlotIndex(entries: PartyEntry[]): number | null {
    if (!entries.length || this.cursorOnCancel()) {
      return null;
    }
    const clamped = Math.max(0, Math.min(this.cursorIndex, entries.length - 1));
    return entries[clamped].index;
  }

  private syncWramState(): void {
    const entries = this.partyEntries();
    const wram = this.gameState.wram;
    wram.wPartyCount = entries.length;
    wram.wPartyMenuCursor = this.cursorOnCancel() ? entries.length + 1 : Math.max(1, this.cursorIndex + 1);
    wram.wPartyMenuActionText = this.action & 0xff;
    wram.wMenuCursorY = wram.wPartyMenuCursor;
    wram.w2DMenuNumRows = entries.length + 1;
    wram.wSwitchMon = this.switchOrigin === null ? 0 : this.switchOrigin + 1;
    wram.wMonType = MonType.PARTYMON;
    const entry = this.currentEntry();
    if (!entry) {
      wram.wCurPartyMon = 0;
      wram.wCurPartySpecies = "";
    } else {
      wram.wCurPartyMon = entry.index;
      wram.wCurPartySpecies = String(entry.pokemon.species?.id ?? "").toUpperCase();
    }
    if (!this.submenuChoices.length) {
      wram.wMonSubmenuCount = 0;
    }
  }

  private currentEntry(): PartyEntry | null {
    const entries = this.partyEntries();
    if (!entries.length) {
      return null;
    }
    if (this.cursorIndex < 0) {
      this.cursorIndex = 0;
    }
    if (this.cursorIndex >= entries.length) {
      return null;
    }
    return entries[this.cursorIndex];
  }

  private selectionTarget(entry: PartyEntry): Pokemon {
    const partyPokemon = this.party.pokemon[entry.index];
    if (!partyPokemon) {
      return entry.pokemon;
    }
    return Object.assign(partyPokemon, {
      _statExpForStat: entry.pokemon._statExpForStat,
      _calculateStat: entry.pokemon._calculateStat,
    }) as Pokemon;
  }

  private cursorOnCancel(): boolean {
    return this.cursorIndex >= this.partyEntries().length;
  }

  private playCursor(): void {}

  private playConfirm(): void {
    this.audioEngine?.playSound?.("menu_option");
  }

  _name_row_y(rowIndex: number): number {
    return partyMenuNameRow(rowIndex);
  }

  _status_row_y(rowIndex: number): number {
    return partyMenuStatusRow(rowIndex);
  }

  _cancel_row_y(entryCount: number): number {
    return partyMenuCancelRow(entryCount);
  }

  _is_egg(pokemon: Pokemon): boolean {
    return String(pokemon.species?.id ?? "").toUpperCase() === "EGG"
      || String(pokemon.nickname ?? "").toUpperCase() === "EGG";
  }

  private placePointerIcons(tilemap: PartyMenuTilemap, entries: PartyEntry[]): void {
    if (this.cursorOnCancel()) {
      const row = this._cancel_row_y(entries.length);
      tilemap.writeText(POINTER_COLUMN, row, "\u25b6", { maxLength: 1 });
    } else {
      const row = this._name_row_y(Math.max(0, Math.min(this.cursorIndex, entries.length - 1)));
      tilemap.writeText(POINTER_COLUMN, row, "\u25b6", { maxLength: 1 });
    }
    if (this.mode === "switch" && this.switchOrigin !== null) {
      const row = this.rowForPartySlot(this.switchOrigin, entries);
      if (row !== null) {
        tilemap.writeText(POINTER_COLUMN, row, "\u25b7", { maxLength: 1 });
      }
    }
  }

  private rowForPartySlot(slotIndex: number, entries: PartyEntry[]): number | null {
    for (let idx = 0; idx < entries.length; idx += 1) {
      if (entries[idx].index === slotIndex) {
        return this._name_row_y(idx);
      }
    }
    return null;
  }

  private placePromptText(tilemap: PartyMenuTilemap, entries: PartyEntry[]): void {
    tilemap.drawWindow(
      TEXTBOX_BORDER_REGION.left,
      TEXTBOX_BORDER_REGION.top,
      TEXTBOX_BORDER_REGION.width,
      TEXTBOX_BORDER_REGION.height,
      { attr: PAL_BG_TEXT },
    );
    const text = entries.length
      ? PARTY_MENU_PROMPTS[actionFromWram(this.gameState.wram.wPartyMenuActionText)]
      : NO_PARTY_TEXT;
    tilemap.writeText(
      TEXTBOX_BORDER_REGION.left + 1,
      TEXTBOX_BORDER_REGION.top + TEXTBOX_BORDER_REGION.height - 2,
      text,
      { maxLength: TEXTBOX_REGION.width, pad: true },
    );
  }

  private buildSubmenuChoices(pokemon: Pokemon): PartyMenuChoice[] {
    const choices: PartyMenuChoice[] = [];
    const wram = this.gameState.wram;
    if (this.battleMenu) {
      // ASM: pokecrystal_disassembly/engine/battle/party_menu.asm (battle submenu items).
      [MonMenuItem.STATS, MonMenuItem.SWITCH, MonMenuItem.CANCEL].forEach((item) => {
        choices.push(this.makeStaticChoice(item));
      });
      return this.finalizeSubmenuChoices(choices);
    }
    if (this._is_egg(pokemon)) {
      [MonMenuItem.STATS, MonMenuItem.SWITCH, MonMenuItem.CANCEL].forEach((item) => {
        choices.push(this.makeStaticChoice(item));
      });
    } else {
      if (wram.wLinkMode === 0) {
        pokemon.moves.forEach((move) => {
          if (!move) {
            return;
          }
          const moveName = String((move as { name?: MoveName }).name ?? "");
          const entry = FIELD_MOVE_LOOKUP[moveName];
          if (!entry) {
            return;
          }
          const label = this.formatMoveLabel(String(entry.value));
          choices.push({ item: entry.item, label, moveEntry: entry });
        });
      }
      [MonMenuItem.STATS, MonMenuItem.SWITCH, MonMenuItem.MOVE].forEach((item) => {
        if (choices.length >= NUM_MON_SUBMENU_ITEMS) {
          return;
        }
        choices.push(this.makeStaticChoice(item));
      });
      if (choices.length < NUM_MON_SUBMENU_ITEMS && wram.wLinkMode === 0) {
        const itemChoice = this.itemIsMail(pokemon.item) ? MonMenuItem.MAIL : MonMenuItem.ITEM;
        choices.push(this.makeStaticChoice(itemChoice));
      }
      if (choices.length < NUM_MON_SUBMENU_ITEMS) {
        choices.push(this.makeStaticChoice(MonMenuItem.CANCEL));
      }
    }

    return this.finalizeSubmenuChoices(choices);
  }

  private finalizeSubmenuChoices(choices: PartyMenuChoice[]): PartyMenuChoice[] {
    const wram = this.gameState.wram;
    const limited = choices.slice(0, NUM_MON_SUBMENU_ITEMS);
    const count = limited.length;
    wram.wMonSubmenuCount = count;
    if (wram.wMonSubmenuItems.length < NUM_MON_SUBMENU_ITEMS + 1) {
      wram.wMonSubmenuItems = Array(NUM_MON_SUBMENU_ITEMS + 1).fill(0);
    }
    for (let idx = 0; idx < NUM_MON_SUBMENU_ITEMS + 1; idx += 1) {
      if (idx < count) {
        wram.wMonSubmenuItems[idx] = limited[idx].item;
      } else if (idx === count) {
        wram.wMonSubmenuItems[idx] = -1;
      } else {
        wram.wMonSubmenuItems[idx] = 0;
      }
    }

    const bottom = SUBMENU_REGION.top + SUBMENU_REGION.height - 1;
    wram.wMenuBorderBottomCoord = bottom;
    const top = 1 + bottom - 2 * (count + 1);
    wram.wMenuBorderTopCoord = top;
    this.submenuRowStart = wram.wMenuBorderTopCoord + 2;
    return limited;
  }

  private makeStaticChoice(item: MonMenuItem): PartyMenuChoice {
    const entry = MENU_OPTION_LOOKUP[item];
    let label = MonMenuItem[item] ?? String(item);
    if (entry && typeof entry.value === "number" && entry.value in MON_MENU_OPTION_STRINGS) {
      label = MON_MENU_OPTION_STRINGS[entry.value];
    }
    label = label.replace(/_/g, " ");
    return { item, label, moveEntry: entry ?? null };
  }

  private formatMoveLabel(moveName: string): string {
    return moveName.replace(/_/g, " ");
  }

  private itemIsMail(itemName: string | null | undefined): boolean {
    if (!itemName) {
      return false;
    }
    return String(itemName).toUpperCase().includes("MAIL");
  }

  private writeSubmenuTilemap(tilemap: PartyMenuTilemap): void {
    if (!this.submenuChoices.length) {
      return;
    }
    const top = this.gameState.wram.wMenuBorderTopCoord;
    const bottom = this.gameState.wram.wMenuBorderBottomCoord;
    const height = Math.max(2, bottom - top + 1);
    tilemap.drawWindow(SUBMENU_REGION.left, top, SUBMENU_REGION.width, height, { attr: PAL_BG_TEXT });
    const innerWidth = Math.max(0, SUBMENU_REGION.width - 3);
    const pointerCol = SUBMENU_REGION.left + 1;
    const textCol = SUBMENU_REGION.left + 2;
    let row = this.submenuRowStart;
    this.submenuChoices.forEach((choice, index) => {
      const pointer = index === this.submenuIndex ? "\u25b6" : " ";
      tilemap.writeText(pointerCol, row, pointer, { maxLength: 1, pad: false });
      tilemap.writeText(textCol, row, choice.label, { maxLength: innerWidth, pad: true });
      row += 2;
    });
  }

  private writeGiveTakeTilemap(tilemap: PartyMenuTilemap): void {
    tilemap.drawWindow(
      GIVE_TAKE_REGION.left,
      GIVE_TAKE_REGION.top,
      GIVE_TAKE_REGION.width,
      GIVE_TAKE_REGION.height,
      { attr: PAL_BG_TEXT },
    );
    const pointerCol = GIVE_TAKE_REGION.left + 1;
    const textCol = GIVE_TAKE_REGION.left + 3;
    const options = ["GIVE", "TAKE"];
    options.forEach((label, idx) => {
      const row = GIVE_TAKE_REGION.top + 1 + idx;
      const pointer = idx === this.giveTakeIndex ? "\u25b6" : " ";
      tilemap.writeText(pointerCol, row, pointer, { maxLength: 1, pad: false });
      tilemap.writeText(textCol, row, label, { maxLength: 4, pad: true });
    });
  }

  private buildTilemap(entries?: PartyEntry[]): PartyMenuTilemap {
    const tilemap = new PartyMenuTilemap();
    const resolvedEntries = entries ?? this.partyEntries();
    const qualities =
      PARTY_MENU_QUALITY_POINTERS[this.action] ?? PARTY_MENU_QUALITY_POINTERS[PartyMenuAction.CHOOSE_POKEMON];
    this.qualityRenderer.apply(this, tilemap, resolvedEntries, qualities);
    this.placePointerIcons(tilemap, resolvedEntries);
    this.placePromptText(tilemap, resolvedEntries);
    if (this.mode === "submenu") {
      this.writeSubmenuTilemap(tilemap);
    } else if (this.mode === "give_take") {
      this.writeGiveTakeTilemap(tilemap);
    }
    return tilemap;
  }

  handleInput(event: GameEngineEvent): SelectionResult | null {
    if (this.itemSelectionInProgress) {
      return null;
    }
    if (!isKeyDownEvent(event) && !isKeyUpEvent(event)) {
      return null;
    }
    const hram = this.gameState.hram.joypad;
    hram.hJoyDown = 0;
    hram.hJoypadDown = 0;
    if (!isKeyDownEvent(event)) {
      updateJoypadStateFromKeys(hram, []);
      return null;
    }

    const padPressed = this.latchMenuJoypad(event);
    let result: SelectionResult | null = null;
    if (this.mode === "list") {
      result = this.handleListInput(padPressed);
    } else if (this.mode === "submenu") {
      result = this.handleSubmenuInput(padPressed);
    } else if (this.mode === "switch") {
      result = this.handleSwitchInput(padPressed);
    } else if (this.mode === "give_take") {
      result = this.handleGiveTakeInput(padPressed);
    }
    this.syncWramState();
    return result;
  }

  private handleListInput(padPressed: number): SelectionResult | null {
    const entries = this.partyEntries();
    const optionCount = Math.max(1, entries.length + 1);
    if (padPressed & B_PAD_UP) {
      this.cursorIndex = (this.cursorIndex - 1 + optionCount) % optionCount;
      this.playCursor();
    } else if (padPressed & B_PAD_DOWN) {
      this.cursorIndex = (this.cursorIndex + 1) % optionCount;
      this.playCursor();
    } else if (padPressed & B_PAD_A) {
      if (this.cursorOnCancel()) {
        this.playConfirm();
        return ["cancel", -1];
      }
      if (entries.length) {
        this.playConfirm();
        const entry = entries[this.cursorIndex];
        if (this.selectionHandler) {
          const shouldClear = this.selectionHandler(this.selectionTarget(entry), entry.index);
          if (shouldClear) {
            this.clearSelectionRequest();
          }
        } else {
          this.openSubmenu(entry);
        }
      }
    } else if (padPressed & B_PAD_B) {
      this.playConfirm();
      if (this.selectionCancel) {
        this.selectionCancel();
        this.clearSelectionRequest();
      }
      return ["cancel", -1];
    }
    return null;
  }

  private handleSubmenuInput(padPressed: number): SelectionResult | null {
    if (!this.submenuChoices.length || !this.submenuEntry) {
      this.closeSubmenu();
      return null;
    }
    const totalChoices = this.submenuChoices.length;
    const wram = this.gameState.wram;
    if (padPressed & B_PAD_UP) {
      this.submenuIndex = (this.submenuIndex - 1 + totalChoices) % totalChoices;
      wram.wMenuCursorY = this.submenuIndex + 1;
      this.playCursor();
      return null;
    }
    if (padPressed & B_PAD_DOWN) {
      this.submenuIndex = (this.submenuIndex + 1) % totalChoices;
      wram.wMenuCursorY = this.submenuIndex + 1;
      this.playCursor();
      return null;
    }
    if (padPressed & B_PAD_B) {
      this.closeSubmenu();
      this.playConfirm();
      return null;
    }
    if (padPressed & B_PAD_A) {
      const choice = this.submenuChoices[this.submenuIndex];
      wram.wMenuCursorY = this.submenuIndex + 1;
      wram.wMenuSelection = choice.item;
      this.playConfirm();
      this.executeMonMenuChoice(choice, this.submenuEntry);
    }
    return null;
  }

  private handleGiveTakeInput(padPressed: number): SelectionResult | null {
    if (!this.itemMenuEntry) {
      this.closeGiveTakeMenu();
      return null;
    }
    if (padPressed & (B_PAD_UP | B_PAD_DOWN)) {
      this.giveTakeIndex = 1 - this.giveTakeIndex;
      this.playCursor();
      return null;
    }
    if (padPressed & B_PAD_B) {
      this.closeGiveTakeMenu();
      this.playConfirm();
      return null;
    }
    if (padPressed & B_PAD_A) {
      this.playConfirm();
      if (this.giveTakeIndex === 0) {
        this.giveItemToPokemon(this.itemMenuEntry);
      } else {
        this.takeItemFromPokemon(this.itemMenuEntry);
      }
      if (!this.itemSelectionInProgress) {
        this.closeGiveTakeMenu();
      }
    }
    return null;
  }

  private openSubmenu(entry: PartyEntry): void {
    this.submenuEntry = entry;
    this.submenuChoices = this.buildSubmenuChoices(entry.pokemon);
    if (!this.submenuChoices.length) {
      this.closeSubmenu();
      return;
    }
    this.submenuIndex = 0;
    this.mode = "submenu";
    this.gameState.wram.wMenuCursorY = 1;
    this.freezeMonIcons();
  }

  private closeSubmenu(): void {
    this.submenuChoices = [];
    this.submenuEntry = null;
    this.submenuIndex = 0;
    this.gameState.wram.wMenuSelection = 0;
    if (this.mode === "submenu") {
      this.mode = "list";
    }
    this.unfreezeMonIcons();
  }

  private freezeMonIcons(): void {
    this.iconsFrozen = true;
  }

  private unfreezeMonIcons(): void {
    this.iconsFrozen = false;
  }

  private executeMonMenuChoice(choice: PartyMenuChoice, entry: PartyEntry): void {
    if (choice.item === MonMenuItem.CANCEL) {
      this.closeSubmenu();
      return;
    }
    if (choice.item === MonMenuItem.SWITCH) {
      this.closeSubmenu();
      if (this.switchBehavior === "swap") {
        if (this.partyEntries().length < 2) {
          return;
        }
        this.beginSwitchMode(entry.index);
        return;
      }
      const handler = this.actionHandlers.get(choice.item);
      if (!handler) {
        throw new Error(`Mon menu action ${MonMenuItem[choice.item] ?? choice.item} has no handler registered`);
      }
      handler(this, entry.pokemon, entry.index);
      return;
    }
    if (choice.item === MonMenuItem.ITEM) {
      this.closeSubmenu();
      this.openGiveTakeMenu(entry);
      return;
    }
    const handler = this.actionHandlers.get(choice.item);
    if (!handler) {
      throw new Error(`Mon menu action ${MonMenuItem[choice.item] ?? choice.item} has no handler registered`);
    }
    this.closeSubmenu();
    handler(this, entry.pokemon, entry.index);
  }

  private beginSwitchMode(originIndex: number): void {
    if (this.switchBehavior === "swap") {
      this.switchModeAction = this.action;
      this.action = PartyMenuAction.MOVE;
    }
    this.mode = "switch";
    this.switchOrigin = originIndex;
    this.cursorIndex = originIndex;
  }

  private openGiveTakeMenu(entry: PartyEntry): void {
    if (this._is_egg(entry.pokemon)) {
      throw new Error("Eggs cannot hold or receive items.");
    }
    this.itemMenuEntry = entry;
    this.giveTakeIndex = 0;
    this.mode = "give_take";
    this.gameState.wram.menu_sprite_anims_enabled = true;
  }

  private closeGiveTakeMenu(): void {
    this.itemMenuEntry = null;
    if (this.mode === "give_take") {
      this.mode = "list";
    }
    this.gameState.wram.menu_sprite_anims_enabled = false;
  }

  private giveItemToPokemon(entry: PartyEntry): void {
    const pokemon = entry.pokemon;
    const selection = this.promptItemSelection(pokemon);
    if (this.itemSelectionInProgress) {
      return;
    }
    if (!selection) {
      return;
    }
    this.finalizeGiveItemSelection(entry, selection);
  }

  private finalizeGiveItemSelection(entry: PartyEntry, selection: string): void {
    const pokemon = entry.pokemon;
    try {
      const heldItem = pokemon.item;
      let addedOldItem = false;
      if (heldItem) {
        if (!this.itemSystem.addItem(heldItem)) {
          throw new Error("Bag is full; cannot store the held item.");
        }
        addedOldItem = true;
      }
      if (!this.itemSystem.removeItem(selection)) {
        if (addedOldItem && heldItem) {
          this.itemSystem.removeItem(heldItem);
        }
        throw new Error(`The bag no longer contains ${selection.replace(/_/g, " ")}.`);
      }
      pokemon.item = selection;
      this.closeGiveTakeMenu();
    } finally {
      this.itemSelectionInProgress = false;
    }
  }

  private takeItemFromPokemon(entry: PartyEntry): void {
    const pokemon = entry.pokemon;
    const heldItem = pokemon.item;
    if (!heldItem) {
      throw new Error("This Pokémon is not holding any item.");
    }
    if (!this.itemSystem.addItem(heldItem)) {
      throw new Error("Bag is full; cannot take the item.");
    }
    pokemon.item = undefined;
  }

  private promptItemSelection(pokemon: Pokemon): string | null {
    if (!this.itemSelector) {
      return null;
    }
    const selection = this.itemSelector(pokemon);
    if (typeof (selection as PromiseLike<string | null>)?.then === "function") {
      this.itemSelectionInProgress = true;
      const pendingEntry = this.findPartyEntryForPokemon(pokemon);
      if (!pendingEntry) {
        this.itemSelectionInProgress = false;
        return null;
      }
      (selection as PromiseLike<string | null>).then(
        (selectedValue) => {
          const canonical = this.canonicalizeItemSelection(selectedValue);
          if (canonical) {
            this.finalizeGiveItemSelection(pendingEntry, canonical);
            return;
          }
          this.itemSelectionInProgress = false;
          this.closeGiveTakeMenu();
        },
        (error) => {
          this.itemSelectionInProgress = false;
          this.closeGiveTakeMenu();
          throw error;
        },
      );
      return null;
    }
    if (selection == null) {
      return null;
    }
    if (typeof selection !== "string") {
      throw new Error("Synchronous item selection must resolve to a string or null.");
    }
    return this.canonicalizeItemSelection(selection);
  }

  private canonicalizeItemSelection(selection: string | null): string | null {
    const canonical = String(selection ?? "").replace(/ /g, "_").toUpperCase();
    return canonical.length ? canonical : null;
  }

  private findPartyEntryForPokemon(target: Pokemon): PartyEntry | null {
    const entries = this.partyEntries();
    for (const entry of entries) {
      if (entry.pokemon === target) {
        return entry;
      }
    }
    return null;
  }

  private handleSwitchInput(padPressed: number): SelectionResult | null {
    const entries = this.partyEntries();
    const optionCount = entries.length;
    if (!optionCount) {
      this.mode = "list";
      this.switchOrigin = null;
      if (this.switchModeAction !== null) {
        this.action = this.switchModeAction;
        this.switchModeAction = null;
      }
      return null;
    }
    if (padPressed & B_PAD_B) {
      this.mode = "list";
      this.switchOrigin = null;
      if (this.switchModeAction !== null) {
        this.action = this.switchModeAction;
        this.switchModeAction = null;
      }
      this.playConfirm();
      return null;
    }
    if (padPressed & B_PAD_UP) {
      this.cursorIndex = (this.cursorIndex - 1 + optionCount) % optionCount;
      this.playCursor();
    } else if (padPressed & B_PAD_DOWN) {
      this.cursorIndex = (this.cursorIndex + 1) % optionCount;
      this.playCursor();
    } else if (padPressed & B_PAD_A) {
      const entry = this.currentEntry();
      if (!entry || this.switchOrigin === null) {
        this.mode = "list";
        if (this.switchModeAction !== null) {
          this.action = this.switchModeAction;
          this.switchModeAction = null;
        }
        return null;
      }
      if (entry.index !== this.switchOrigin) {
        this.swapPartyMembers(this.switchOrigin, entry.index);
      }
      this.mode = "list";
      this.switchOrigin = null;
      if (this.switchModeAction !== null) {
        this.action = this.switchModeAction;
        this.switchModeAction = null;
      }
      this.playConfirm();
    }
    return null;
  }

  private swapPartyMembers(source: number, target: number): void {
    const party = this.party.pokemon;
    [party[source], party[target]] = [party[target], party[source]];
  }

  private latchMenuJoypad(event: GameEngineEvent): number {
    const key = normalizeKey(event);
    updateJoypadStateFromKeys(this.gameState.hram.joypad, key ? [key] : []);
    const joypad = this.gameState.hram.joypad;
    this.gameState.wram.wMenuJoypad = joypad.hJoyDown;
    return joypad.hJoyPressed;
  }

  draw(): void {
    if (!this.screen) {
      return;
    }
    this.updateHpAnimations();
    const entries = this.partyEntries();
    const tilemap = this.buildTilemap(entries);
    this.screen.fill([255, 255, 255, 255]);
    tilemap.blit(this.screen, this.tileset);
    this.bgMapWriter.request(tilemap);
    this.drawPartyIcons(entries);
  }

  private drawPartyIcons(entries: PartyEntry[]): void {
    if (!this.screen) {
      return;
    }
    const highlightSlot = this.highlightSlotIndex(entries);
    const switchMode = this.mode === "switch";
    const switchOrigin = switchMode ? this.switchOrigin : null;
    this.iconRenderer.draw(this.screen, entries, {
      frozen: this.iconsFrozen,
      highlightSlot,
      switchOriginSlot: switchOrigin,
      switchMode,
    });
  }

  private updateHpAnimations(): void {
    if (!this.hpAnimations.size) {
      return;
    }
    for (const [index, state] of this.hpAnimations.entries()) {
      state.step();
      if (!state.active) {
        this.hpAnimations.delete(index);
      }
    }
  }
}
