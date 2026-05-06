// ASM mapping: engine/pokedex/pokedex.asm (menu state machine + WRAM mirrors).
import { gameEngine } from "../game-engine";
import { Surface } from "../surface";
import { pokemonData } from "@pokecrystal/assets/data";
import type { MenuUI } from "./types";
import { NUM_POKEMON, NUM_UNOWN, LANDMARK_SPECIAL } from "../../core/constants";
import { DexMode } from "../../core/enums/pokedex";
import { GameState } from "../../core/state";
import { pokedexFlagSet } from "../../core/pokedex";
import { getWorldMapLocation } from "../../core/home";
import { PrintOption } from "../../core/enums/ui-enums";
import { AudioEngine } from "../../engine/systems/audio";
import { TILE_SIZE } from "../../engine/world/tile";
import { isButtonEvent, isCancelEvent, isConfirmEvent, isKeyDownEvent, KeyEvent, GameButton } from "../../input/buttons";
import { PokedexEntryData, parsePokedexEntryFile } from "./pokedex-entry-loader";
import {
  LIST_WINDOW_LENGTH,
  SCREEN_HEIGHT_TILES,
  SCREEN_WIDTH_TILES,
  SEARCH_TYPE_SEQUENCE,
  typeDisplayName,
  drawPokedexList,
  drawSearchResultsWindow,
  drawSearchScreen,
  drawSearchTypeNotFoundMessage,
} from "./pokedex-layout";
import {
  drawMainSidebar,
  drawSearchResultsBackground,
  drawEntryPage,
  drawPokedexCursorOverlay,
  drawOptionScreen,
  drawSearchSlowpoke,
  drawUnownModeScreen,
  ensurePokedexTiles,
  formatHeight,
  formatWeight,
  UNOWN_LETTER_COORDS,
  UNOWN_LETTER_WORDS,
} from "./pokedex-render";
import {
  findAdjacentSeenDexEntryIndex,
  listingMoveDown,
  listingMoveUp,
  listingPageDown,
  listingPageUp,
  orderEntriesForMode,
  restoreSearchListingFromBackup,
  stepArrowCursorClamp,
  DexEntry,
} from "./pokedex-state";
import { PokedexSearchController, PokedexUnownModeController } from "./pokedex-behaviors";
import { CursorOAMEntry, PokedexCursorOAM, type PokedexCursorVariant } from "./pokedex-cursor";
import { TownMapOverlay } from "../overlays/town-map-overlay";
import type { PokemonSpecies } from "../../core/models";

const MODE_CHANGE_DELAY_FRAMES = 128;
const SEARCH_RESULTS_HEIGHT = 4;
const POKEDEX_SCX = 5;
const ARROW_CURSOR_DELAY_FRAMES = 12;
const SEARCH_TYPE_NOT_FOUND_DELAY_FRAMES = 0x80;

export enum DexScreenState {
  MAIN = "MAIN",
  ENTRY = "ENTRY",
  OPTIONS = "OPTIONS",
  SEARCH = "SEARCH",
  SEARCH_RESULTS = "SEARCH_RESULTS",
  UNOWN = "UNOWN",
}

const legendLines = (firstLine: string, ...rest: string[]): string[] => {
  return [firstLine, ...rest];
};

export const buildPokedexControlLines = (state: DexScreenState): string[] => {
  switch (state) {
    case DexScreenState.MAIN:
      return legendLines(
        "D-Pad=Move L/R=Page A=Entry",
        "Start=Search Select=Options B=Exit"
      );
    case DexScreenState.ENTRY:
      return legendLines("Up/Down=Prev/Next L/R=Action", "A=Select B=Back");
    case DexScreenState.OPTIONS:
      return legendLines("Up/Down=Move A/Start=Select", "B/Select=Back");
    case DexScreenState.SEARCH:
      return legendLines("Up/Down=Move L/R=Type A=Confirm Start=Back");
    case DexScreenState.SEARCH_RESULTS:
      return legendLines("Up/Down=Move L/R=Page A=Entry B=Back");
    case DexScreenState.UNOWN:
      return legendLines("L/R=Move A/B/Select=Back");
    default:
      return legendLines("B=Back");
  }
};

const MODE_CHANGE_MESSAGE: [string, string] = ["Changing modes.", "Please wait."];
const ENTRY_ACTIONS = ["PAGE", "AREA", "CRY", "PRNT"];
const ENTRY_ACTION_COORDINATES: Array<[number, number]> = [
  [1, 2],
  [6, 7],
  [11, 12],
  [15, 16],
];

export enum DexJumptableState {
  MAIN_SCR = 0,
  UPDATE_MAIN_SCR = 1,
  DEX_ENTRY_SCR = 2,
  UPDATE_DEX_ENTRY_SCR = 3,
  REINIT_DEX_ENTRY_SCR = 4,
  SEARCH_SCR = 5,
  UPDATE_SEARCH_SCR = 6,
  OPTION_SCR = 7,
  UPDATE_OPTION_SCR = 8,
  SEARCH_RESULTS_SCR = 9,
  UPDATE_SEARCH_RESULTS_SCR = 10,
  UNOWN_MODE = 11,
  UPDATE_UNOWN_MODE = 12,
  EXIT = 13,
}

const DexJumptableState_VALUES = new Set(Object.values(DexJumptableState));

const SCREEN_TO_INIT_JUMPTABLE_STATE: Record<DexScreenState, DexJumptableState> = {
  [DexScreenState.MAIN]: DexJumptableState.MAIN_SCR,
  [DexScreenState.ENTRY]: DexJumptableState.DEX_ENTRY_SCR,
  [DexScreenState.OPTIONS]: DexJumptableState.OPTION_SCR,
  [DexScreenState.SEARCH]: DexJumptableState.SEARCH_SCR,
  [DexScreenState.SEARCH_RESULTS]: DexJumptableState.SEARCH_RESULTS_SCR,
  [DexScreenState.UNOWN]: DexJumptableState.UNOWN_MODE,
};

const SCREEN_TO_UPDATE_JUMPTABLE_STATE: Record<DexScreenState, DexJumptableState> = {
  [DexScreenState.MAIN]: DexJumptableState.UPDATE_MAIN_SCR,
  [DexScreenState.ENTRY]: DexJumptableState.UPDATE_DEX_ENTRY_SCR,
  [DexScreenState.OPTIONS]: DexJumptableState.UPDATE_OPTION_SCR,
  [DexScreenState.SEARCH]: DexJumptableState.UPDATE_SEARCH_SCR,
  [DexScreenState.SEARCH_RESULTS]: DexJumptableState.UPDATE_SEARCH_RESULTS_SCR,
  [DexScreenState.UNOWN]: DexJumptableState.UPDATE_UNOWN_MODE,
};

type SpeciesData = PokemonSpecies;

type DataLoader = {
  getPokemonSpecies?: (id: string) => SpeciesData | null;
  get_pokemon_species?: (id: string) => SpeciesData | null;
  getSpecies?: (id: string) => SpeciesData | null;
  pokemonData?: Map<string, SpeciesData> | Record<string, SpeciesData | undefined>;
  pokemon_data?: Record<string, SpeciesData | undefined>;
  speciesMap?: Map<string, SpeciesData> | Record<string, SpeciesData | undefined>;
};

type ScriptRunner = {
  defer?: (script: string) => void;
};

type Printer = {
  printDexEntry: (
    speciesId: string,
    pokedexNumber: number,
    entryData: PokedexEntryData,
    printOption: PrintOption
  ) => void;
};

const formatPrintOption = (option: PrintOption): string => {
  return PrintOption[option] ?? String(option);
};

const renderPokedexPrintOutput = (
  speciesId: string,
  pokedexNumber: number,
  entryData: PokedexEntryData,
  options: PrintOption,
): string => {
  const header = `${speciesId}  #${String(pokedexNumber).padStart(3, "0")}`;
  const metrics = `${entryData.classification} | HT:${entryData.heightDigits} WT:${entryData.weightDigits} | MODE:${formatPrintOption(options)}`;
  const body = entryData.pages.map((page) => page.replace(/@/g, "\n")).join("\n\n");
  return `${header}\n${metrics}\n\n${body}\n`;
};

export const __test__renderPokedexPrintOutput = renderPokedexPrintOutput;

const writePokedexPrintToStorage = (filename: string, content: string): void => {
  if (typeof localStorage !== "undefined") {
    try {
      localStorage.setItem(`pokecrystal-ts:pokedex-print:${filename}`, content);
      return;
    } catch {
      // Intentionally ignore storage write failures.
    }
  }
  if (typeof console !== "undefined") {
    console.log(content);
  }
};

export class DefaultPokedexPrinter implements Printer {
  printDexEntry(
    speciesId: string,
    pokedexNumber: number,
    entryData: PokedexEntryData,
    printOption: PrintOption
  ): void {
    const output = renderPokedexPrintOutput(
      speciesId,
      pokedexNumber,
      entryData,
      printOption
    );
    const filename = `${String(pokedexNumber).padStart(3, "0")}_${speciesId.toLowerCase()}.txt`;
    writePokedexPrintToStorage(filename, output);
  }
}

export class PokedexHardwareRegisters {
  public scx = 0;
  public scy = 0;
  public wx = 0;
  public wy = 0;
  public lcdcPointer: number | null = null;
}

export class HardwareRegisterStack {
  public current = new PokedexHardwareRegisters();
  private readonly stack: PokedexHardwareRegisters[] = [];

  push(): void {
    this.stack.push({ ...this.current });
  }

  pop(): void {
    if (!this.stack.length) {
      throw new Error("Attempted to restore Pok\u00e9dex registers without any saved state.");
    }
    this.current = this.stack.pop()!;
  }

  updateTop(updater: (registers: PokedexHardwareRegisters) => void): void {
    if (!this.stack.length) {
      throw new Error("No saved Pok\u00e9dex registers available to update.");
    }
    updater(this.stack[this.stack.length - 1]);
  }

  reset(): void {
    this.stack.length = 0;
    this.current = new PokedexHardwareRegisters();
  }

  get depth(): number {
    return this.stack.length;
  }
}

const resolveSpeciesMaps = (
  dataLoader?: DataLoader | null,
): [Record<string, SpeciesData>, Record<number, SpeciesData>] => {
  const byName: Record<string, SpeciesData> = {};
  const byNumber: Record<number, SpeciesData> = {};
  const map =
    dataLoader?.pokemonData ??
    dataLoader?.pokemon_data ??
    dataLoader?.speciesMap ??
    null;
  if (map instanceof Map && map.size) {
    for (const [name, species] of map.entries()) {
      byName[name] = species;
      const dexNumber = Number(species.int_id);
      if (Number.isFinite(dexNumber) && dexNumber > 0) {
        byNumber[dexNumber] = species;
      }
    }
    return [byName, byNumber];
  }
  if (map && !(map instanceof Map)) {
    for (const [name, species] of Object.entries(map)) {
      if (!species) {
        continue;
      }
      byName[name] = species;
      const dexNumber = Number(species.int_id);
      if (Number.isFinite(dexNumber) && dexNumber > 0) {
        byNumber[dexNumber] = species;
      }
    }
    if (Object.keys(byName).length > 0) {
      return [byName, byNumber];
    }
  }
  for (const species of pokemonData as SpeciesData[]) {
    if (!species?.id) {
      continue;
    }
    byName[species.id] = species;
    const dexNumber = Number(species.int_id);
    if (Number.isFinite(dexNumber) && dexNumber > 0) {
      byNumber[dexNumber] = species;
    }
  }
  if (Object.keys(byName).length > 0) {
    return [byName, byNumber];
  }
  throw new Error("Pok\u00e9dex requires species data to render entries.");
};

export const __test__resolveSpeciesMaps = resolveSpeciesMaps;

const flagSetFromBytes = pokedexFlagSet;

export class PokedexScreen {
  public state: DexScreenState = DexScreenState.MAIN;

  private readonly searchController: PokedexSearchController;
  private readonly cursorOam: PokedexCursorOAM;
  private readonly unownController: PokedexUnownModeController;
  private readonly registerStack = new HardwareRegisterStack();
  private jumptableState: DexJumptableState = DexJumptableState.MAIN_SCR;
  private modeChangeCooldown = 0;
  private pendingOptionReturn = false;
  private modeChangeMessage: [string, string] | null = null;
  private searchResultsPending = false;
  private pendingModeTarget: DexMode | null = null;
  private modeChangeSoundPlayed = false;

  private speciesByName: Record<string, SpeciesData>;
  private speciesByNumber: Record<number, SpeciesData>;
  private orderedEntries: DexEntry[] = [];
  private entryCache: Record<string, PokedexEntryData> = {};
  private entryPageIndex = 0;
  private entryReturnState: DexScreenState = DexScreenState.MAIN;
  private entrySourceList: DexEntry[] = [];
  private entrySourceIndex = 0;
  private pendingEntryOrder: [DexMode, DexEntry[], number, number[]] | null = null;

  private searchCursor = 0;
  private searchTypeIndexes = [1, 0];
  private searchResults: DexEntry[] = [];
  private searchResultsCursor = 0;
  private searchResultsScrollOffset = 0;
  private searchTypeNotFoundDelay = 0;
  private mainListingEnd = 0;

  private get entryActionIndex(): number {
    return this.gameState.wram.wDexArrowCursorPosIndex;
  }

  private set entryActionIndex(value: number) {
    const maxIndex = ENTRY_ACTIONS.length - 1;
    const clamped = Math.max(0, Math.min(value, maxIndex));
    const state = this.gameState.wram;
    if (clamped !== state.wDexArrowCursorPosIndex) {
      state.wDexArrowCursorPosIndex = clamped;
      this.resetArrowCursorCounters();
    }
  }

  private readonly areaOverlay: TownMapOverlay;
  private readonly printer: Printer;
  private readonly audioEngine: AudioEngine | null;

  constructor(
    private readonly ui: MenuUI,
    private readonly gameState: GameState,
    options?: {
      dataLoader?: DataLoader | null;
      audioEngine?: AudioEngine | null;
      scriptRunner?: ScriptRunner | null;
      printer?: Printer | null;
    },
  ) {
    this.searchController = new PokedexSearchController(gameState);
    this.cursorOam = new PokedexCursorOAM();
    this.unownController = new PokedexUnownModeController(gameState);
    this.printer = options?.printer ?? new DefaultPokedexPrinter();
    this.audioEngine = options?.audioEngine ?? null;
    this.areaOverlay = new TownMapOverlay(this.ui, this.gameState, {
      script_runner: null,
      lock_movement: this.noOp,
      unlock_movement: this.noOp,
    });
    ensurePokedexTiles(this.ui);
    const [byName, byNumber] = resolveSpeciesMaps(options?.dataLoader ?? null);
    this.speciesByName = byName;
    this.speciesByNumber = byNumber;
    this.reset();
  }

  reset(): void {
    this.gameState.hram.hInMenu = 1;
    this.registerStack.reset();
    this.jumptableState = DexJumptableState.MAIN_SCR;
    this.modeChangeCooldown = 0;
    this.pendingOptionReturn = false;
    this.modeChangeMessage = null;
    this.entryPageIndex = 0;
    this.entryReturnState = DexScreenState.MAIN;
    this.entrySourceList = [];
    this.entrySourceIndex = 0;
    this.searchCursor = 0;
    this.searchTypeIndexes = [1, 0];
    this.searchResults = [];
    this.searchResultsCursor = 0;
    this.searchResultsScrollOffset = 0;
    this.searchTypeNotFoundDelay = 0;
    this.mainListingEnd = 0;
    this.cursorOam.entries = [];
    this.searchController.resetSlowpokeAnimation();
    this.refreshCurrentLocation();
    this.pendingEntryOrder = null;

    const state = this.gameState.wram;
    state.wDexListingHeight = LIST_WINDOW_LENGTH;
    state.wDexListingCursor = 0;
    state.wDexListingScrollOffset = 0;
    state.wDexListingEnd = 0;
    state.wDexListingCursorBackup = 0;
    state.wDexListingScrollOffsetBackup = 0;
    state.wDexArrowCursorPosIndex = 0;
    state.wDexSearchResultCount = 0;
    state.wPrevDexEntry = 0;
    state.wUnlockedUnownMode = Boolean(this.gameState.sram.unown_dex);
    state.wCurDexMode = state.wLastDexMode;
    this.orderEntries(state.wCurDexMode);

    state.wDexListingCursor = 0;
    state.wDexListingScrollOffset = 0;
    state.wDexListingHeight = LIST_WINDOW_LENGTH;
    this.mainListingEnd = state.wDexListingEnd;
    state.wDexListingEnd = this.mainListingEnd;
    this.entryActionIndex = 0;
    this.optionCursorIndex = 0;
    this.setScreenState(DexScreenState.MAIN, DexJumptableState.MAIN_SCR);

    if (this.areaOverlay.visible) {
      this.areaOverlay.close();
    }
  }

  getControlLines(): string[] {
    return buildPokedexControlLines(this.state);
  }

  private noOp(): void {
    return;
  }

  setJumptableState(state: DexJumptableState): void {
    this.jumptableState = state;
    this.gameState.wram.wJumptableIndex = Number(state);
  }

  private screenInitJumptableState(screen: DexScreenState): DexJumptableState {
    return SCREEN_TO_INIT_JUMPTABLE_STATE[screen] ?? DexJumptableState.MAIN_SCR;
  }

  private configureRegistersForScreen(screen: DexScreenState): void {
    const registers = this.registerStack.current;
    registers.scy = 0;
    registers.lcdcPointer = null;
    registers.wy = 0;
    if (screen === DexScreenState.MAIN) {
      registers.scx = POKEDEX_SCX;
      registers.wx = this.gameState.wram.wCurDexMode === DexMode.OLD ? 0x4a : 0x47;
    } else if (screen === DexScreenState.SEARCH_RESULTS) {
      registers.scx = POKEDEX_SCX;
      registers.wx = 0x4a;
    } else if ([DexScreenState.ENTRY, DexScreenState.OPTIONS, DexScreenState.SEARCH].includes(screen)) {
      registers.scx = 0;
      registers.wx = 0xa7;
    } else if (screen === DexScreenState.UNOWN) {
      registers.scx = 0;
      registers.wx = 0;
    } else {
      registers.scx = 0;
      registers.wx = 0;
    }
    this.applyCurrentRegisters();
  }

  private applyCurrentRegisters(): void {
    const registers = this.registerStack.current;
    const hram = this.gameState.hram;
    hram.hSCX = registers.scx & 0xff;
    hram.hWX = registers.wx & 0xff;
    hram.hWY = registers.wy & 0xff;
  }

  private setScreenState(screen: DexScreenState, jumptableState: DexJumptableState, options?: { configureRegisters?: boolean }): void {
    this.state = screen;
    if (options?.configureRegisters ?? true) {
      this.initializeScreen(screen);
      this.configureRegistersForScreen(screen);
    } else {
      this.applyCurrentRegisters();
    }
    this.setJumptableState(jumptableState);
  }

  private setUpdateStateForCurrentScreen(): void {
    const updateState = SCREEN_TO_UPDATE_JUMPTABLE_STATE[this.state];
    if (updateState !== undefined) {
      this.setJumptableState(updateState);
    }
  }

  private enterModalState(screen: DexScreenState, jumptableState: DexJumptableState): void {
    this.registerStack.push();
    this.setScreenState(screen, jumptableState);
  }

  private restoreModalState(screen: DexScreenState, options?: { previousJumptable?: DexJumptableState | null }): void {
    const hasSavedRegisters = this.registerStack.depth > 0;
    if (hasSavedRegisters) {
      this.registerStack.pop();
    } else {
      this.registerStack.reset();
    }
    const jumptableState = options?.previousJumptable ?? this.screenInitJumptableState(screen);
    this.setScreenState(screen, jumptableState, { configureRegisters: !hasSavedRegisters });
  }

  private initializeScreen(screen: DexScreenState): void {
    switch (screen) {
      case DexScreenState.MAIN:
        this.initializeMainScreen();
        return;
      case DexScreenState.ENTRY:
        this.initializeEntryScreen();
        return;
      case DexScreenState.SEARCH:
        this.initializeSearchScreen();
        return;
      case DexScreenState.SEARCH_RESULTS:
        this.initializeSearchResultsScreen();
        return;
      case DexScreenState.OPTIONS:
        this.initializeOptionScreen();
        return;
      case DexScreenState.UNOWN:
        this.initializeUnownScreen();
        return;
      default:
        throw new Error(`Pok\u00e9dex screen initializer missing for ${screen}.`);
    }
  }

  private initializeMainScreen(): void {
    const state = this.gameState.wram;
    const listingEnd = Math.max(this.orderedEntries.length, this.mainListingEnd);
    this.mainListingEnd = listingEnd;
    state.wDexListingHeight = LIST_WINDOW_LENGTH;
    state.wDexListingEnd = listingEnd;
    state.wDexSearchResultCount = 0;
    this.resetArrowCursorCounters();
    this.clampCursor();
  }

  private initializeEntryScreen(): void {
    this.entryPageIndex = Math.max(0, this.entryPageIndex);
    this.entryActionIndex = Math.max(0, Math.min(this.entryActionIndex, ENTRY_ACTIONS.length - 1));
    this.resetArrowCursorCounters();
  }

  private initializeSearchScreen(): void {
    this.gameState.wram.wDexListingHeight = LIST_WINDOW_LENGTH;
    this.searchCursor = 0;
    this.searchTypeNotFoundDelay = 0;
    this.cursorOam.entries = [];
    this.searchController.resetSlowpokeAnimation();
    this.resetArrowCursorCounters();
  }

  private initializeSearchResultsScreen(): void {
    const state = this.gameState.wram;
    state.wDexListingHeight = SEARCH_RESULTS_HEIGHT;
    state.wDexListingCursor = 0;
    state.wDexListingScrollOffset = 0;
    this.searchResultsCursor = 0;
    this.searchResultsScrollOffset = 0;
    this.resetArrowCursorCounters();
  }

  private initializeOptionScreen(): void {
    this.resetArrowCursorCounters();
  }

  private initializeUnownScreen(): void {
    const letters = this.unownLetterSlots();
    const state = this.gameState.wram;
    const letterCount = letters.length;
    if (state.wDexUnownCount !== letterCount) {
      this.unownController.initUnownMode(letterCount);
    } else if (letterCount <= 0) {
      state.wDexCurUnownIndex = 0;
    } else {
      state.wDexCurUnownIndex = Math.min(state.wDexCurUnownIndex, letterCount - 1);
    }
    this.resetArrowCursorCounters();
  }

  private closeOptionsScreen(): void {
    this.pendingOptionReturn = false;
    this.modeChangeMessage = null;
    this.restoreModalState(DexScreenState.MAIN);
  }

  private returnFromEntry(): void {
    const previousValue = this.gameState.wram.wPrevDexEntryJumptableIndex;
    const previousState =
      DexJumptableState_VALUES.has(previousValue as DexJumptableState)
        ? (previousValue as DexJumptableState)
        : this.screenInitJumptableState(this.entryReturnState);
    this.restoreModalState(this.entryReturnState, { previousJumptable: previousState });
  }

  handleInput(event: KeyEvent): string | null {
    if (!isKeyDownEvent(event)) {
      return null;
    }
    this.setUpdateStateForCurrentScreen();
    if (this.state === DexScreenState.MAIN) {
      return this.handleMainInput(event);
    }
    if (this.state === DexScreenState.ENTRY) {
      return this.handleEntryInput(event);
    }
    if (this.state === DexScreenState.OPTIONS) {
      return this.handleOptionInput(event);
    }
    if (this.state === DexScreenState.SEARCH) {
      return this.handleSearchInput(event);
    }
    if (this.state === DexScreenState.SEARCH_RESULTS) {
      return this.handleSearchResultsInput(event);
    }
    if (this.state === DexScreenState.UNOWN) {
      return this.handleUnownInput(event);
    }
    return null;
  }

  private handleMainInput(event: KeyEvent): string | null {
    const total = this.orderedEntries.length;
    const key = event.key;
    if (key === gameEngine.K_UP) {
      this.moveCursorUp();
    } else if (key === gameEngine.K_DOWN) {
      this.moveCursorDown();
    } else if (key === gameEngine.K_LEFT) {
      this.pageUp();
    } else if (key === gameEngine.K_RIGHT) {
      this.pageDown();
    } else if (isConfirmEvent(event)) {
      return this.enterEntryView();
    } else if (isCancelEvent(event)) {
      this.recordExitMode();
      return "exit";
    } else if (total === 0) {
      if (isButtonEvent(event, GameButton.Select)) {
        this.openOptionScreen();
      } else if (isButtonEvent(event, GameButton.Start)) {
        this.openSearchScreen();
      }
    } else if (isButtonEvent(event, GameButton.Select)) {
      this.openOptionScreen();
    } else if (isButtonEvent(event, GameButton.Start)) {
      this.openSearchScreen();
    }
    return null;
  }

  private handleEntryInput(event: KeyEvent): string | null {
    if (this.areaOverlay.visible) {
      this.areaOverlay.handle_input?.(event);
      if (!this.areaOverlay.visible) {
        this.redisplayEntryScreen();
      }
      return null;
    }
    const key = event.key;
    if (key === gameEngine.K_UP) {
      this.moveEntryToAdjacent(-1);
      return null;
    }
    if (key === gameEngine.K_DOWN) {
      this.moveEntryToAdjacent(1);
      return null;
    }
    if (key === gameEngine.K_LEFT) {
      this.moveEntryActionCursor(-1);
      return null;
    }
    if (key === gameEngine.K_RIGHT) {
      this.moveEntryActionCursor(1);
      return null;
    }
    if (isConfirmEvent(event)) {
      this.dispatchEntryAction();
    } else if (isCancelEvent(event)) {
      this.returnFromEntry();
    }
    return null;
  }

  private handleOptionInput(event: KeyEvent): string | null {
    if (this.modeChangeCooldown > 0) {
      return null;
    }
    const modes = this.optionModes();
    if (!modes.length) {
      throw new Error("No Pok\u00e9dex modes are available to render.");
    }
    let cursor = Math.min(this.optionCursorIndex, modes.length - 1);
    if (event.key === gameEngine.K_UP) {
      [cursor] = stepArrowCursorClamp(cursor, -1, modes.length);
    } else if (event.key === gameEngine.K_DOWN) {
      [cursor] = stepArrowCursorClamp(cursor, 1, modes.length);
    } else if (isConfirmEvent(event)) {
      this.applyOptionSelection(modes[cursor]);
      return null;
    } else if (isButtonEvent(event, GameButton.Select) || isCancelEvent(event)) {
      this.closeOptionsScreen();
      return null;
    }
    this.optionCursorIndex = cursor;
    return null;
  }

  private handleSearchInput(event: KeyEvent): string | null {
    if (this.searchController.isSlowpokeAnimationActive() || this.searchTypeNotFoundDelay > 0) {
      return null;
    }
    const key = event.key;
    if (key === gameEngine.K_UP) {
      this.searchCursor = Math.max(0, this.searchCursor - 1);
      return null;
    }
    if (key === gameEngine.K_DOWN) {
      this.searchCursor = Math.min(3, this.searchCursor + 1);
      return null;
    }
    if (key === gameEngine.K_LEFT) {
      if (this.searchCursor === 0 || this.searchCursor === 1) {
        this.stepSearchType(this.searchCursor, -1);
      }
      return null;
    }
    if (key === gameEngine.K_RIGHT) {
      if (this.searchCursor === 0 || this.searchCursor === 1) {
        this.stepSearchType(this.searchCursor, 1);
      }
      return null;
    }
    if (isConfirmEvent(event)) {
      if (this.searchCursor === 2) {
        this.startSearchQuery();
      } else if (this.searchCursor === 3) {
        this.exitSearchScreen();
      }
      return null;
    }
    if (isCancelEvent(event)) {
      this.exitSearchScreen();
      return null;
    }
    if (isButtonEvent(event, GameButton.Start)) {
      this.exitSearchScreen();
    }
    return null;
  }

  private handleSearchResultsInput(event: KeyEvent): string | null {
    const key = event.key;
    const total = this.searchResults.length;
    if (key === gameEngine.K_UP && total) {
      this.moveSearchCursor(-1);
      return null;
    }
    if (key === gameEngine.K_DOWN && total) {
      this.moveSearchCursor(1);
      return null;
    }
    if (key === gameEngine.K_LEFT && total) {
      this.moveSearchResultsPage(-1);
      return null;
    }
    if (key === gameEngine.K_RIGHT && total) {
      this.moveSearchResultsPage(1);
      return null;
    }
    if (isConfirmEvent(event) && total) {
      return this.enterEntryView();
    }
    if (isCancelEvent(event)) {
      this.returnToSearchScreen();
    }
    return null;
  }

  private handleUnownInput(event: KeyEvent): string | null {
    const key = event.key;
    if (key === gameEngine.K_LEFT) {
      this.unownController.moveCursor(-1);
      return null;
    }
    if (key === gameEngine.K_RIGHT) {
      this.unownController.moveCursor(1);
      return null;
    }
    if (isConfirmEvent(event) || isCancelEvent(event) || isButtonEvent(event, GameButton.Select)) {
      this.exitUnownMode();
    }
    return null;
  }

  private stepSearchType(index: number, delta: number): void {
    const maxIndex = SEARCH_TYPE_SEQUENCE.length - 1;
    let selection = this.searchTypeIndexes[index] + delta;
    if (index === 0) {
      if (selection < 1) {
        selection = maxIndex;
      } else if (selection > maxIndex) {
        selection = 1;
      }
    } else {
      if (selection < 0) {
        selection = maxIndex;
      } else if (selection > maxIndex) {
        selection = 0;
      }
    }
    this.searchTypeIndexes[index] = selection;
  }

  private startSearchQuery(): void {
    this.searchTypeNotFoundDelay = 0;
    const [type1, type2] = this.searchController.configureTypeIndexes(this.searchTypeIndexes);
    const state = this.gameState.wram;
    this.mainListingEnd = state.wDexListingEnd;
    state.wDexListingScrollOffsetBackup = state.wDexListingScrollOffset;
    state.wDexListingCursorBackup = state.wDexListingCursor;
    state.wPrevDexEntryBackup = state.wPrevDexEntry;
    const results = this.searchController.filterEntries(
      this.orderedEntries,
      flagSetFromBytes(this.gameState.sram.pokedex_owned),
      type1,
      type2,
    );
    this.searchResults = results;
    this.searchResultsCursor = 0;
    this.searchResultsScrollOffset = 0;
    state.wDexListingEnd = results.length;
    state.wDexSearchResultCount = results.length;
    state.wDexListingHeight = SEARCH_RESULTS_HEIGHT;
    state.wDexListingScrollOffset = 0;
    state.wDexListingCursor = 0;
    this.searchResultsPending = true;
    this.searchController.startSlowpokeAnimation();
  }

  private completeSearchAnimation(): void {
    if (!this.searchResultsPending) {
      return;
    }
    this.searchResultsPending = false;
    if (this.gameState.wram.wDexSearchResultCount === 0) {
      this.orderEntries(this.gameState.wram.wCurDexMode);
      const state = this.gameState.wram;
      this.searchTypeNotFoundDelay = SEARCH_TYPE_NOT_FOUND_DELAY_FRAMES;
      this.searchResults = [];
      this.searchResultsCursor = 0;
      this.searchResultsScrollOffset = 0;
      this.searchCursor = 0;
      state.wDexListingHeight = LIST_WINDOW_LENGTH;
      this.resetArrowCursorCounters();
      return;
    }
    this.setScreenState(DexScreenState.SEARCH_RESULTS, DexJumptableState.SEARCH_RESULTS_SCR);
  }

  private playReadText2Sound(): void {
    if (!this.audioEngine) {
      return;
    }
    try {
      this.audioEngine.playSound("SFX_READ_TEXT_2");
    } catch {
      return;
    }
  }

  private setSearchResultsCursor(cursor: number, scroll: number): void {
    const state = this.gameState.wram;
    const total = this.searchResults.length;
    const height = Math.max(1, state.wDexListingHeight);
    const maxScroll = Math.max(0, total - height);
    const clampedCursor = total > 0 ? Math.max(0, Math.min(cursor, total - 1)) : 0;
    const clampedScroll = total > 0 ? Math.max(0, Math.min(scroll, maxScroll)) : 0;
    this.searchResultsCursor = clampedCursor;
    this.searchResultsScrollOffset = clampedScroll;
    state.wDexListingCursor = clampedCursor;
    state.wDexListingScrollOffset = clampedScroll;
  }

  private moveSearchResultsPage(direction: -1 | 1): void {
    const state = this.gameState.wram;
    const total = this.searchResults.length;
    if (!total) {
      return;
    }
    const movement =
      direction < 0
        ? listingPageUp(this.searchResultsCursor, state.wDexListingScrollOffset, state.wDexListingHeight)
        : listingPageDown(
          this.searchResultsCursor,
          state.wDexListingScrollOffset,
          state.wDexListingHeight,
          total,
        );
    const [cursor, scroll, changed] = movement;
    if (changed) {
      this.setSearchResultsCursor(cursor, scroll);
    }
  }

  private moveSearchCursor(delta: number): void {
    const total = this.searchResults.length;
    if (!total) {
      return;
    }
    let cursor = this.searchResultsCursor + delta;
    cursor = Math.max(0, Math.min(cursor, total - 1));
    let scroll = this.searchResultsScrollOffset;
    if (cursor < scroll) {
      scroll = cursor;
    } else if (cursor >= scroll + this.gameState.wram.wDexListingHeight) {
      scroll = cursor - this.gameState.wram.wDexListingHeight + 1;
    }
    const maxScroll = Math.max(0, total - this.gameState.wram.wDexListingHeight);
    this.setSearchResultsCursor(cursor, Math.max(0, Math.min(scroll, maxScroll)));
  }

  private enterEntryView(): string | null {
    const entry = this.currentEntry;
    if (!entry) {
      return null;
    }
    const seenSet = flagSetFromBytes(this.gameState.sram.pokedex_seen);
    if (!seenSet.has(entry.species.int_id)) {
      return null;
    }
    this.getEntryData(entry);
    this.entryPageIndex = 0;
    const currentState = this.state;
    this.entryReturnState = currentState;
    const prevJumptable = this.screenInitJumptableState(currentState);
    this.gameState.wram.wPrevDexEntryJumptableIndex = Number(prevJumptable);
    if (currentState === DexScreenState.SEARCH_RESULTS) {
      this.entrySourceList = this.searchResults;
      this.entrySourceIndex = this.searchResultsCursor;
    } else {
      this.entrySourceList = this.orderedEntries;
      this.entrySourceIndex = this.currentListIndex();
    }
    this.entryActionIndex = 0;
    this.enterModalState(DexScreenState.ENTRY, DexJumptableState.DEX_ENTRY_SCR);
    this.gameState.wram.wPrevDexEntry = entry.pokedexNumber;
    return null;
  }

  private openOptionScreen(): void {
    const modes = this.optionModes();
    if (!modes.length) {
      throw new Error("Option screen cannot open without any modes.");
    }
    this.enterModalState(DexScreenState.OPTIONS, DexJumptableState.OPTION_SCR);
    const index = modes.indexOf(this.gameState.wram.wCurDexMode);
    this.optionCursorIndex = index >= 0 ? index : 0;
  }

  private openSearchScreen(): void {
    this.searchTypeIndexes = [1, 0];
    this.searchController.configureTypeIndexes(this.searchTypeIndexes);
    this.enterModalState(DexScreenState.SEARCH, DexJumptableState.SEARCH_SCR);
    this.searchCursor = 0;
    this.searchResultsPending = false;
    this.searchResults = [];
    this.searchResultsCursor = 0;
    this.searchResultsScrollOffset = 0;
    this.searchTypeNotFoundDelay = 0;
    this.searchController.resetSlowpokeAnimation();
  }

  private returnToSearchScreen(): void {
    const state = this.gameState.wram;
    restoreSearchListingFromBackup(state);
    this.searchResults = [];
    this.searchResultsCursor = 0;
    this.searchResultsScrollOffset = 0;
    this.searchTypeNotFoundDelay = 0;
    this.orderEntries(state.wCurDexMode);
    this.setScreenState(DexScreenState.SEARCH, DexJumptableState.SEARCH_SCR);
  }

  private exitSearchScreen(): void {
    const state = this.gameState.wram;
    state.wDexListingHeight = LIST_WINDOW_LENGTH;
    state.wDexListingEnd = this.mainListingEnd;
    state.wDexListingScrollOffset = state.wDexListingScrollOffsetBackup;
    state.wDexListingCursor = state.wDexListingCursorBackup;
    state.wDexSearchResultCount = 0;
    this.searchResultsPending = false;
    this.searchTypeNotFoundDelay = 0;
    this.searchController.resetSlowpokeAnimation();
    this.restoreModalState(DexScreenState.MAIN);
  }

  private enterUnownMode(): void {
    this.enterModalState(DexScreenState.UNOWN, DexJumptableState.UNOWN_MODE);
  }

  private exitUnownMode(): void {
    this.restoreModalState(DexScreenState.OPTIONS);
  }

  private applyOptionSelection(mode: DexMode): void {
    if (mode === DexMode.UNOWN) {
      this.unownController.ensureUnlocked();
      this.enterUnownMode();
      return;
    }
    if (mode === this.gameState.wram.wCurDexMode) {
      this.closeOptionsScreen();
      return;
    }
    const preparedEntries = this.prepareEntriesForMode(mode);
    this.pendingEntryOrder = [mode, ...preparedEntries];
    this.pendingModeTarget = mode;
    this.modeChangeCooldown = MODE_CHANGE_DELAY_FRAMES;
    this.modeChangeSoundPlayed = false;
    this.pendingOptionReturn = true;
    this.modeChangeMessage = MODE_CHANGE_MESSAGE;
  }

  private recordExitMode(): void {
    this.gameState.wram.wLastDexMode = this.gameState.wram.wCurDexMode;
    this.gameState.hram.hInMenu = 0;
    this.playReadText2Sound();
    this.setJumptableState(DexJumptableState.EXIT);
    this.registerStack.reset();
    this.applyCurrentRegisters();
    this.pendingOptionReturn = false;
    this.modeChangeMessage = null;
  }

  private refreshCurrentLocation(): void {
    const state = this.gameState.wram;
    let location = getWorldMapLocation(state.wMapGroup, state.wMapNumber);
    if (location === LANDMARK_SPECIAL) {
      location = getWorldMapLocation(state.wBackupMapGroup, state.wBackupMapNumber);
    }
    state.wDexCurLocation = location;
  }

  private get currentEntry(): DexEntry | null {
    let entries: DexEntry[];
    let cursor: number;
    if (this.state === DexScreenState.SEARCH_RESULTS) {
      entries = this.searchResults;
      cursor = this.searchResultsCursor;
    } else if (this.state === DexScreenState.ENTRY) {
      entries = this.entrySourceList;
      cursor = this.entrySourceIndex;
    } else {
      entries = this.orderedEntries;
      cursor = this.currentListIndex();
    }
    if (!entries.length) {
      return null;
    }
    if (cursor < 0 || cursor >= entries.length) {
      return null;
    }
    return entries[cursor];
  }

  private get currentEntryData(): PokedexEntryData | null {
    const entry = this.currentEntry;
    if (!entry) {
      return null;
    }
    return this.entryCache[entry.species.id] ?? null;
  }

  private getEntryData(entry: DexEntry): PokedexEntryData {
    const speciesId = entry.species.id;
    if (!this.entryCache[speciesId]) {
      this.entryCache[speciesId] = parsePokedexEntryFile(speciesId);
    }
    return this.entryCache[speciesId];
  }

  private resetArrowCursorCounters(): void {
    const state = this.gameState.wram;
    state.wDexArrowCursorDelayCounter = ARROW_CURSOR_DELAY_FRAMES;
    state.wDexArrowCursorBlinkCounter = 0;
  }

  private get optionCursorIndex(): number {
    return this.gameState.wram.wDexArrowCursorPosIndex;
  }

  private set optionCursorIndex(value: number) {
    const state = this.gameState.wram;
    const clamped = Math.max(0, value);
    if (clamped !== state.wDexArrowCursorPosIndex) {
      state.wDexArrowCursorPosIndex = clamped;
      this.resetArrowCursorCounters();
    }
  }

  // entryActionIndex is backed by wDexArrowCursorPosIndex (shared with option cursor).

  private optionModes(): DexMode[] {
    const modes = [DexMode.NEW, DexMode.OLD, DexMode.ABC];
    if (this.gameState.wram.wUnlockedUnownMode) {
      modes.push(DexMode.UNOWN);
    }
    return modes;
  }

  private unownLetterSlots(): number[] {
    const slots: number[] = [];
    const maxSlots = UNOWN_LETTER_COORDS.length;
    const raw = this.gameState.wram.wUnownDex.slice(0, maxSlots);
    for (const value of raw) {
      if (value <= 0) {
        break;
      }
      slots.push(Math.min(NUM_UNOWN, value));
    }
    return slots;
  }

  private unownWordForLetter(letterValue: number): string {
    if (letterValue >= 1 && letterValue < UNOWN_LETTER_WORDS.length) {
      return UNOWN_LETTER_WORDS[letterValue];
    }
    return "";
  }

  private unownSpeciesId(letterValue: number): string | null {
    if (letterValue >= 1 && letterValue <= NUM_UNOWN) {
      return `unown_${String.fromCharCode("a".charCodeAt(0) + letterValue - 1)}`;
    }
    return null;
  }

  private orderEntries(mode: DexMode): void {
    const [entries, listingEnd, pokedexOrder] = this.prepareEntriesForMode(mode);
    this.applyEntryOrder(mode, entries, listingEnd, pokedexOrder);
  }

  private prepareEntriesForMode(mode: DexMode): [DexEntry[], number, number[]] {
    const [entries, listingEnd] = orderEntriesForMode(this.gameState, this.speciesByName, this.speciesByNumber, mode);
    if (!entries.length && mode !== DexMode.ABC) {
      throw new Error("Pok\u00e9dex ordering returned no entries.");
    }
    const buffer = entries.map((entry) => entry.pokedexNumber);
    if (buffer.length < NUM_POKEMON) {
      buffer.push(...Array(NUM_POKEMON - buffer.length).fill(0));
    }
    return [entries, listingEnd, buffer.slice(0, NUM_POKEMON)];
  }

  private applyEntryOrder(mode: DexMode, entries: DexEntry[], listingEnd: number, pokedexOrder: number[]): void {
    const state = this.gameState.wram;
    state.wCurDexMode = mode;
    state.wDexListingEnd = listingEnd;
    this.mainListingEnd = listingEnd;
    state.wPokedexOrder = pokedexOrder.slice(0, NUM_POKEMON);
    this.orderedEntries = entries;
    this.restorePreviousEntryCursor();
    this.clampCursor();
  }

  private restorePreviousEntryCursor(): void {
    const prev = this.gameState.wram.wPrevDexEntry;
    if (prev <= 0) {
      return;
    }
    for (let index = 0; index < this.orderedEntries.length; index += 1) {
      if (this.orderedEntries[index].pokedexNumber === prev) {
        this.setListCursorAt(index);
        return;
      }
    }
  }

  private moveCursorUp(): void {
    const state = this.gameState.wram;
    const [cursor, scroll, changed] = listingMoveUp(state.wDexListingCursor, state.wDexListingScrollOffset);
    if (changed) {
      state.wDexListingCursor = cursor;
      state.wDexListingScrollOffset = scroll;
    }
  }

  private moveCursorDown(): void {
    const state = this.gameState.wram;
    const accessibleEnd = this.orderedEntries.length;
    const [cursor, scroll, changed] = listingMoveDown(
      state.wDexListingCursor,
      state.wDexListingScrollOffset,
      state.wDexListingHeight,
      accessibleEnd,
    );
    if (changed) {
      state.wDexListingCursor = cursor;
      state.wDexListingScrollOffset = scroll;
    }
  }

  private pageUp(): void {
    const state = this.gameState.wram;
    const [cursor, scroll, changed] = listingPageUp(
      state.wDexListingCursor,
      state.wDexListingScrollOffset,
      state.wDexListingHeight,
    );
    if (changed) {
      state.wDexListingCursor = cursor;
      state.wDexListingScrollOffset = scroll;
    }
  }

  private pageDown(): void {
    const state = this.gameState.wram;
    const [cursor, scroll, changed] = listingPageDown(
      state.wDexListingCursor,
      state.wDexListingScrollOffset,
      state.wDexListingHeight,
      this.orderedEntries.length,
    );
    if (changed) {
      state.wDexListingCursor = cursor;
      state.wDexListingScrollOffset = scroll;
    }
  }

  private clampCursor(): void {
    const state = this.gameState.wram;
    const total = this.accessibleLength();
    if (total <= 0) {
      state.wDexListingCursor = 0;
      state.wDexListingScrollOffset = 0;
      return;
    }
    const index = this.currentListIndex();
    const height = Math.max(1, state.wDexListingHeight);
    const maxScroll = Math.max(0, total - height);
    let scroll = Math.max(0, Math.min(state.wDexListingScrollOffset, maxScroll));
    if (index < scroll) {
      scroll = index;
    } else if (index >= scroll + height) {
      scroll = Math.min(index - height + 1, maxScroll);
    }
    const cursor = index - scroll;
    state.wDexListingScrollOffset = scroll;
    state.wDexListingCursor = cursor;
  }

  private accessibleLength(): number {
    return this.orderedEntries.length;
  }

  private currentListIndex(): number {
    const total = this.orderedEntries.length;
    if (total <= 0) {
      return 0;
    }
    const state = this.gameState.wram;
    const index = state.wDexListingScrollOffset + state.wDexListingCursor;
    return Math.max(0, Math.min(index, total - 1));
  }

  private setListCursorAt(index: number): void {
    const state = this.gameState.wram;
    const total = this.orderedEntries.length;
    if (total <= 0) {
      state.wDexListingCursor = 0;
      state.wDexListingScrollOffset = 0;
      return;
    }
    const height = Math.max(1, state.wDexListingHeight);
    const maxIndex = total - 1;
    const clamped = Math.max(0, Math.min(index, maxIndex));
    const maxScroll = Math.max(0, total - height);
    const scroll = Math.min(Math.max(clamped - height + 1, 0), maxScroll);
    const cursor = clamped - scroll;
    state.wDexListingScrollOffset = scroll;
    state.wDexListingCursor = cursor;
  }

  private tickModeChange(): void {
    if (this.modeChangeCooldown <= 0) {
      return;
    }
    this.modeChangeCooldown -= 1;
    if (!this.modeChangeSoundPlayed && this.modeChangeCooldown === Math.floor(MODE_CHANGE_DELAY_FRAMES / 2)) {
      this.playModeChangeSound();
      this.modeChangeSoundPlayed = true;
    }
    if (this.modeChangeCooldown === 0 && this.pendingOptionReturn) {
      const pendingMode = this.pendingModeTarget ?? this.gameState.wram.wCurDexMode;
      if (this.pendingEntryOrder) {
        const [mode, entries, listingEnd, pokedexOrder] = this.pendingEntryOrder;
        this.applyEntryOrder(mode, entries, listingEnd, pokedexOrder);
        this.pendingEntryOrder = null;
      } else {
        this.orderEntries(pendingMode);
      }
      this.pendingModeTarget = null;
      this.updateSavedRegistersForMode(this.gameState.wram.wCurDexMode);
      this.closeOptionsScreen();
    }
  }

  private tickSearchTypeNotFoundDelay(): void {
    if (this.searchTypeNotFoundDelay > 0) {
      this.searchTypeNotFoundDelay -= 1;
    }
  }

  private playModeChangeSound(): void {
    if (!this.audioEngine) {
      return;
    }
    try {
      this.audioEngine.playSound("SFX_CHANGE_DEX_MODE");
    } catch {
      return;
    }
  }

  private updateSavedRegistersForMode(mode: DexMode): void {
    this.registerStack.updateTop((registers) => {
      registers.scx = POKEDEX_SCX;
      registers.wx = mode === DexMode.OLD ? 0x4a : 0x47;
      registers.wy = 0;
    });
  }

  private tickArrowCursorDelay(): void {
    const state = this.gameState.wram;
    if (state.wDexArrowCursorDelayCounter > 0) {
      state.wDexArrowCursorDelayCounter -= 1;
    }
  }

  private layerPixelSize(): [number, number] {
    return [SCREEN_WIDTH_TILES * TILE_SIZE, SCREEN_HEIGHT_TILES * TILE_SIZE];
  }

  private blitLayer(
    dest: Surface,
    source: Surface,
    destX: number,
    destY: number,
    area?: { x: number; y: number; width: number; height: number },
  ): void {
    const [destWidth, destHeight] = dest.get_size();
    const [sourceWidth, sourceHeight] = source.get_size();
    let srcX = area?.x ?? 0;
    let srcY = area?.y ?? 0;
    let width = area?.width ?? sourceWidth;
    let height = area?.height ?? sourceHeight;

    if (srcX < 0) {
      width += srcX;
      destX -= srcX;
      srcX = 0;
    }
    if (srcY < 0) {
      height += srcY;
      destY -= srcY;
      srcY = 0;
    }
    if (srcX + width > sourceWidth) {
      width = sourceWidth - srcX;
    }
    if (srcY + height > sourceHeight) {
      height = sourceHeight - srcY;
    }
    if (destX < 0) {
      const shift = -destX;
      destX = 0;
      srcX += shift;
      width -= shift;
    }
    if (destY < 0) {
      const shift = -destY;
      destY = 0;
      srcY += shift;
      height -= shift;
    }
    if (destX + width > destWidth) {
      width = destWidth - destX;
    }
    if (destY + height > destHeight) {
      height = destHeight - destY;
    }
    if (width <= 0 || height <= 0) {
      return;
    }

    if (source.getCanvasImageSource()) {
      dest.blit(source, [destX, destY], { x: srcX, y: srcY, width, height });
      return;
    }

    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const [r, g, b, a] = source.get_at([srcX + x, srcY + y]);
        if (a === 0) {
          continue;
        }
        dest.set_at([destX + x, destY + y], [r, g, b, a]);
      }
    }
  }

  private blitScrolledLayer(dest: Surface, source: Surface, scrollX: number, scrollY: number): void {
    const [sourceWidth, sourceHeight] = source.get_size();
    const [destWidth, destHeight] = dest.get_size();
    if (!sourceWidth || !sourceHeight) {
      return;
    }
    const withinX = scrollX >= 0 && scrollX + destWidth <= sourceWidth;
    const withinY = scrollY >= 0 && scrollY + destHeight <= sourceHeight;
    if (withinX && withinY) {
      this.blitLayer(dest, source, 0, 0, { x: scrollX, y: scrollY, width: destWidth, height: destHeight });
      return;
    }

    const normX = ((scrollX % sourceWidth) + sourceWidth) % sourceWidth;
    const normY = ((scrollY % sourceHeight) + sourceHeight) % sourceHeight;
    const primaryWidth = Math.min(sourceWidth - normX, destWidth);
    const primaryHeight = Math.min(sourceHeight - normY, destHeight);
    this.blitLayer(dest, source, 0, 0, { x: normX, y: normY, width: primaryWidth, height: primaryHeight });
    const remainingWidth = destWidth - primaryWidth;
    const remainingHeight = destHeight - primaryHeight;
    if (remainingWidth > 0) {
      this.blitLayer(dest, source, primaryWidth, 0, { x: 0, y: normY, width: remainingWidth, height: primaryHeight });
    }
    if (remainingHeight > 0) {
      this.blitLayer(dest, source, 0, primaryHeight, { x: normX, y: 0, width: primaryWidth, height: remainingHeight });
    }
    if (remainingWidth > 0 && remainingHeight > 0) {
      this.blitLayer(dest, source, primaryWidth, primaryHeight, { x: 0, y: 0, width: remainingWidth, height: remainingHeight });
    }
  }

  // ASM: hSCX/hWX scroll + window overlays are composited after tilemaps update.
  private renderLayeredScreen(drawBackground: (layer: Surface) => void, drawWindow: (layer: Surface) => void): void {
    const screen = this.ui.screen;
    if (!screen) {
      return;
    }
    const [screenWidth, screenHeight] = this.layerPixelSize();
    if (screen.get_width() !== screenWidth || screen.get_height() !== screenHeight) {
      throw new Error(
        `Pok\u00e9dex requires a ${screenWidth}x${screenHeight}px screen, got ${screen.get_width()}x${screen.get_height()}.`
      );
    }
    const registers = this.registerStack.current;
    const bgWidthTiles = SCREEN_WIDTH_TILES + Math.ceil(Math.max(registers.scx, 0) / TILE_SIZE);
    const bgWidth = bgWidthTiles * TILE_SIZE;
    const bgLayer = new Surface(bgWidth, screenHeight);
    const windowLayer = new Surface(screenWidth, screenHeight);
    drawBackground(bgLayer);
    drawWindow(windowLayer);

    this.blitScrolledLayer(screen, bgLayer, registers.scx, registers.scy);
    const windowLeft = registers.wx - 7;
    const windowTop = registers.wy;
    if (windowLeft <= -screenWidth || windowLeft >= screenWidth || windowTop <= -screenHeight || windowTop >= screenHeight) {
      return;
    }
    this.blitLayer(screen, windowLayer, windowLeft, windowTop);
  }

  private updateCursorOamForList(
    cursorIndex: number,
    scrollOffset: number,
    listingHeight: number,
    variant: PokedexCursorVariant = "main"
  ): void {
    if (listingHeight <= 0) {
      this.cursorOam.entries = [];
      return;
    }
    this.cursorOam.update(this.gameState.wram.wCurDexMode, cursorIndex, scrollOffset, listingHeight, variant);
  }

  get cursorOamEntries(): CursorOAMEntry[] {
    return [...this.cursorOam.entries];
  }

  draw(): void {
    if (!this.ui.screen) {
      return;
    }
    this.tickArrowCursorDelay();
    this.tickModeChange();
    const state = this.gameState.wram;
    state.wDexArrowCursorBlinkCounter = (state.wDexArrowCursorBlinkCounter + 1) & 0xff;
    const showArrowCursor = (state.wDexArrowCursorBlinkCounter & 0x8) === 0;

    if (this.state === DexScreenState.MAIN) {
      this.drawMainScreen();
      return;
    }
    if (this.state === DexScreenState.SEARCH) {
      this.tickSearchTypeNotFoundDelay();
      this.drawSearchScreen(showArrowCursor);
      if (this.searchController.isSlowpokeAnimationActive()) {
        if (!this.searchController.advanceSlowpokeAnimation()) {
          this.completeSearchAnimation();
        }
      }
      return;
    }
    if (this.state === DexScreenState.SEARCH_RESULTS) {
      this.drawSearchResultsScreen();
      return;
    }
    if (this.state === DexScreenState.ENTRY) {
      this.drawEntryView(showArrowCursor);
      return;
    }
    if (this.state === DexScreenState.OPTIONS) {
      this.drawOptionScreen(showArrowCursor);
      return;
    }
  if (this.state === DexScreenState.UNOWN) {
      this.drawUnownScreen();
      return;
    }
    this.drawMainScreen();
  }

  getTextOverlay(): { viewportLines: string[]; infoLines: string[]; menuLines: string[] } {
    const viewportLines = [`POKEDEX ${this.state}`];
    const seenSet = flagSetFromBytes(this.gameState.sram.pokedex_seen);
    const caughtSet = flagSetFromBytes(this.gameState.sram.pokedex_owned);
    const mode = this.gameState.wram.wCurDexMode;
    const infoLines = this.getControlLines();
    const menuLines: string[] = [];

    if (this.state === DexScreenState.MAIN) {
      viewportLines.push(`MODE: ${this.formatDexModeLabel(mode)}`);
      viewportLines.push(`SEEN: ${seenSet.size}`);
      viewportLines.push(`OWN: ${caughtSet.size}`);
      menuLines.push(
        ...this.buildListingLines(
          this.orderedEntries,
          this.gameState.wram.wDexListingScrollOffset,
          this.gameState.wram.wDexListingCursor,
          this.gameState.wram.wDexListingHeight,
          seenSet,
          caughtSet,
          mode
        )
      );
      return { viewportLines, infoLines, menuLines };
    }

    if (this.state === DexScreenState.SEARCH_RESULTS) {
      viewportLines.push(`RESULTS: ${this.searchResults.length}`);
      menuLines.push(
        ...this.buildListingLines(
          this.searchResults,
          this.searchResultsScrollOffset,
          this.searchResultsCursor,
          SEARCH_RESULTS_HEIGHT,
          seenSet,
          caughtSet,
          mode
        )
      );
      return { viewportLines, infoLines, menuLines };
    }

    if (this.state === DexScreenState.SEARCH) {
      viewportLines.push(`MODE: ${this.formatDexModeLabel(mode)}`);
      const type1 = SEARCH_TYPE_SEQUENCE[this.searchTypeIndexes[0]] ?? null;
      const type2 = SEARCH_TYPE_SEQUENCE[this.searchTypeIndexes[1]] ?? null;
      const type1Label = type1 ? typeDisplayName(type1) : "----";
      const type2Label = type2 ? typeDisplayName(type2) : "----";
      menuLines.push(this.formatCursorLine(`TYPE 1: ${type1Label}`, this.searchCursor === 0));
      menuLines.push(this.formatCursorLine(`TYPE 2: ${type2Label}`, this.searchCursor === 1));
      menuLines.push(this.formatCursorLine("BEGIN SEARCH!!", this.searchCursor === 2));
      menuLines.push(this.formatCursorLine("CANCEL", this.searchCursor === 3));
      return { viewportLines, infoLines, menuLines };
    }

    if (this.state === DexScreenState.OPTIONS) {
      viewportLines.push(`MODE: ${this.formatDexModeLabel(mode)}`);
      const modes = this.optionModes();
      for (let idx = 0; idx < modes.length; idx += 1) {
        const label = this.formatDexModeLabel(modes[idx]);
        menuLines.push(this.formatCursorLine(label, idx === this.optionCursorIndex));
      }
      return { viewportLines, infoLines, menuLines };
    }

    if (this.state === DexScreenState.ENTRY) {
      const entry = this.currentEntry;
      const entryData = this.currentEntryData;
      if (entry) {
        viewportLines.push(`ENTRY: ${entry.species.id} #${String(entry.pokedexNumber).padStart(3, "0")}`);
      }
      if (entryData) {
        viewportLines.push(entryData.classification);
        viewportLines.push(formatHeight(entryData.heightDigits));
        viewportLines.push(formatWeight(entryData.weightDigits));
        viewportLines.push(`PAGE: ${this.entryPageIndex + 1}/${entryData.pages.length}`);
        const pageText = entryData.pages[this.entryPageIndex] ?? "";
        const pageLines = pageText
          .split(" @ ")
          .map((line) => line.trim())
          .filter(Boolean);
        if (pageLines.length) {
          menuLines.push(...pageLines);
        }
      }
      menuLines.push("ACTIONS:");
      for (let idx = 0; idx < ENTRY_ACTIONS.length; idx += 1) {
        menuLines.push(this.formatCursorLine(ENTRY_ACTIONS[idx], idx === this.entryActionIndex));
      }
      return { viewportLines, infoLines, menuLines };
    }

    if (this.state === DexScreenState.UNOWN) {
      const count = Math.max(0, this.gameState.wram.wDexUnownCount ?? 0);
      const index = Math.max(0, Math.min(this.gameState.wram.wDexCurUnownIndex ?? 0, Math.max(0, count - 1)));
      const letter = UNOWN_LETTER_WORDS[index] ?? String(index + 1);
      viewportLines.push(`UNOWN: ${count}`);
      menuLines.push(this.formatCursorLine(`LETTER: ${letter}`, true));
      return { viewportLines, infoLines, menuLines };
    }

    return { viewportLines, infoLines, menuLines };
  }

  private buildListingLines(
    entries: DexEntry[],
    scrollOffset: number,
    cursorIndex: number,
    height: number,
    seenSet: Set<number>,
    caughtSet: Set<number>,
    dexMode: DexMode
  ): string[] {
    if (!entries.length) {
      return ["(no entries)"];
    }
    const windowHeight = Math.max(1, height);
    const maxScroll = Math.max(0, entries.length - windowHeight);
    const scroll = Math.max(0, Math.min(scrollOffset, maxScroll));
    const visible = entries.slice(scroll, scroll + windowHeight);
    const lines: string[] = [];
    if (scroll > 0) {
      lines.push("▲ more above");
    }
    for (let idx = 0; idx < visible.length; idx += 1) {
      const entry = visible[idx];
      const label = this.formatEntryLine(entry, seenSet, caughtSet, dexMode);
      lines.push(this.formatCursorLine(label, idx === cursorIndex));
    }
    if (scroll + visible.length < entries.length) {
      lines.push("▼ more below");
    }
    return lines;
  }

  private formatEntryLine(
    entry: DexEntry,
    seenSet: Set<number>,
    caughtSet: Set<number>,
    dexMode: DexMode
  ): string {
    const seen = seenSet.has(entry.species.int_id);
    const name = seen ? entry.species.id : "-----";
    const numberPrefix = dexMode === DexMode.OLD ? `${String(entry.pokedexNumber).padStart(3, "0")} ` : "";
    const caught = seen && caughtSet.has(entry.species.int_id) ? " *" : "";
    return `${numberPrefix}${name}${caught}`.trimEnd();
  }

  private formatDexModeLabel(mode: DexMode): string {
    return DexMode[mode] ?? String(mode);
  }

  private formatCursorLine(label: string, active: boolean): string {
    const prefix = active ? "\u25b6" : " ";
    return `${prefix} ${label}`;
  }

  private drawMainScreen(): void {
    const ui = this.ui;
    const entry = this.currentEntry;
    const seenSet = flagSetFromBytes(this.gameState.sram.pokedex_seen);
    const caughtSet = flagSetFromBytes(this.gameState.sram.pokedex_owned);
    const sidebarSeen = entry && seenSet.has(entry.species.int_id);
    const speciesId = sidebarSeen && entry ? entry.species.id : null;
    const cursorIndex = this.currentListIndex();
    const scrollOffset = this.gameState.wram.wDexListingScrollOffset;
    this.renderLayeredScreen(
      (bgLayer) => {
        drawMainSidebar(ui, bgLayer, {
          seenCount: seenSet.size,
          caughtCount: caughtSet.size,
          activeSpeciesId: speciesId,
          showQuestionMark: Boolean(entry && !sidebarSeen),
        });
      },
      (windowLayer) => {
        drawPokedexList(
          ui,
          windowLayer,
          this.orderedEntries,
          cursorIndex,
          scrollOffset,
          seenSet,
          caughtSet,
          this.gameState.wram.wCurDexMode,
          this.gameState.wram.wDexListingHeight,
          {
            windowPrompts: true,
            originOffset: [0, 0],
          },
        );
      },
    );
    drawPokedexCursorOverlay(
      ui,
      ui.screen!,
      this.gameState.wram.wCurDexMode,
      cursorIndex,
      scrollOffset,
      this.gameState.wram.wDexListingHeight,
      this.gameState.wram.wDexListingEnd,
    );
    this.updateCursorOamForList(cursorIndex, scrollOffset, this.gameState.wram.wDexListingHeight);
  }

  private drawSearchScreen(showArrowCursor: boolean): void {
    const ui = this.ui;
    drawSearchScreen(
      ui,
      ui.screen!,
      this.searchCursor,
      [this.searchTypeIndexes[0], this.searchTypeIndexes[1]],
      { showArrowCursor },
    );
    if (this.searchController.isSlowpokeAnimationActive()) {
      const frame = this.searchController.currentSlowpokeFrame();
      drawSearchSlowpoke(ui, ui.screen!, frame);
    }
    if (this.searchTypeNotFoundDelay > 0) {
      drawSearchTypeNotFoundMessage(ui, ui.screen!);
    }
  }

  private drawSearchResultsScreen(): void {
    const ui = this.ui;
    const entry = this.currentEntry;
    const seenSet = flagSetFromBytes(this.gameState.sram.pokedex_seen);
    const caughtSet = flagSetFromBytes(this.gameState.sram.pokedex_owned);
    const speciesId = entry && seenSet.has(entry.species.int_id) ? entry.species.id : null;
    this.renderLayeredScreen(
      (bgLayer) => {
        drawSearchResultsBackground(ui, bgLayer, {
          resultCount: this.gameState.wram.wDexSearchResultCount,
          activeSpeciesId: speciesId,
          showQuestionMark: Boolean(entry && !seenSet.has(entry.species.int_id)),
        });
      },
      (windowLayer) => {
        drawSearchResultsWindow(
          ui,
          windowLayer,
          this.searchResults,
          this.searchResultsCursor,
          this.searchResultsScrollOffset,
          [this.searchTypeIndexes[0], this.searchTypeIndexes[1]],
          seenSet,
          caughtSet,
          this.gameState.wram.wCurDexMode,
          this.gameState.wram.wDexListingHeight,
          {
            originOffset: [0, 0],
          },
        );
      },
    );
    drawPokedexCursorOverlay(
      ui,
      ui.screen!,
      this.gameState.wram.wCurDexMode,
      this.searchResultsCursor,
      this.searchResultsScrollOffset,
      this.gameState.wram.wDexListingHeight,
      this.gameState.wram.wDexListingEnd,
      "search_results",
    );
    this.updateCursorOamForList(
      this.searchResultsCursor,
      this.searchResultsScrollOffset,
      this.gameState.wram.wDexListingHeight,
      "search_results",
    );
  }

  private drawOptionScreen(showArrowCursor: boolean): void {
    const ui = this.ui;
    const modes = this.optionModes();
    drawOptionScreen(
      ui,
      ui.screen!,
      modes,
      this.optionCursorIndex,
      this.modeChangeMessage,
      showArrowCursor
    );
  }

  private drawUnownScreen(): void {
    const ui = this.ui;
    const letters = this.unownLetterSlots().slice(0, UNOWN_LETTER_COORDS.length);
    const state = this.gameState.wram;
    let cursorIndex = state.wDexCurUnownIndex;
    cursorIndex = letters.length ? Math.max(0, Math.min(cursorIndex, letters.length - 1)) : 0;
    if (cursorIndex !== state.wDexCurUnownIndex) {
      state.wDexCurUnownIndex = cursorIndex;
    }
    const letterValue = letters[cursorIndex] ?? 0;
    drawUnownModeScreen(ui, ui.screen!, letters, cursorIndex, {
      word: this.unownWordForLetter(letterValue),
      activeSpeciesId: this.unownSpeciesId(letterValue),
    });
  }

  private drawEntryView(showArrowCursor: boolean): void {
    const ui = this.ui;
    const entry = this.currentEntry;
    if (!entry || !ui.screen) {
      return;
    }
    const entryData = this.getEntryData(entry);
    if (!entryData.pages.length) {
      return;
    }
    const pageIndex = Math.min(this.entryPageIndex, entryData.pages.length - 1);
    const caughtSet = flagSetFromBytes(this.gameState.sram.pokedex_owned);
    drawEntryPage(
      ui,
      ui.screen,
      entry,
      entryData,
      pageIndex,
      this.entryActionIndex,
      ENTRY_ACTIONS,
      ENTRY_ACTION_COORDINATES,
      {
        isCaught: caughtSet.has(entry.species.int_id),
        showArrowCursor,
      },
    );
    if (this.areaOverlay.visible) {
      this.areaOverlay.draw?.(ui.screen);
    }
  }

  private advanceEntryPage(): void {
    const entryData = this.currentEntryData;
    if (!entryData) {
      this.returnFromEntry();
      return;
    }
    const lastPageIndex = entryData.pages.length - 1;
    if (this.entryPageIndex < lastPageIndex) {
      this.entryPageIndex += 1;
    } else {
      this.returnFromEntry();
    }
  }

  private moveEntryActionCursor(delta: number): void {
    const [index] = stepArrowCursorClamp(this.entryActionIndex, delta, ENTRY_ACTIONS.length);
    this.entryActionIndex = index;
  }

  private dispatchEntryAction(): void {
    const action = ENTRY_ACTIONS[this.entryActionIndex];
    if (action === "PAGE") {
      this.advanceEntryPage();
    } else if (action === "AREA") {
      this.handleAreaAction();
    } else if (action === "CRY") {
      this.handleCryAction();
    } else if (action === "PRNT") {
      this.handlePrintAction();
    } else {
      throw new Error(`Unknown Pok\u00e9dex action '${action}'.`);
    }
  }

  private moveEntryToAdjacent(delta: number): void {
    const entries = this.entrySourceList;
    if (!entries.length || entries.length <= 1) {
      return;
    }
    const seen = flagSetFromBytes(this.gameState.sram.pokedex_seen);
    const nextIndex = findAdjacentSeenDexEntryIndex(
      entries,
      this.entrySourceIndex,
      delta < 0 ? -1 : 1,
      seen,
    );
    if (nextIndex !== null) {
      this.focusEntryIndex(nextIndex);
    }
  }

  private focusEntryIndex(index: number): void {
    const entries = this.entrySourceList;
    const previousIndex = this.entrySourceIndex;
    if (index < 0 || index >= entries.length) {
      return;
    }
    const entry = entries[index];
    const seen = flagSetFromBytes(this.gameState.sram.pokedex_seen);
    if (!seen.has(entry.species.int_id)) {
      return;
    }
    this.entrySourceIndex = index;
    this.entryPageIndex = 0;
    this.entryActionIndex = 0;
    this.gameState.wram.wPrevDexEntry = entry.pokedexNumber;
    this.getEntryData(entry);
    this.syncEntryListCursor(index);
    if (this.state === DexScreenState.ENTRY && index !== previousIndex) {
      this.setJumptableState(DexJumptableState.REINIT_DEX_ENTRY_SCR);
    }
  }

  private syncEntryListCursor(index: number): void {
    if (this.entryReturnState === DexScreenState.SEARCH_RESULTS) {
      this.syncSearchResultsCursor(index);
      return;
    }
    this.setListCursorAt(index);
  }

  private syncSearchResultsCursor(index: number): void {
    const total = this.searchResults.length;
    if (!total) {
      return;
    }
    const cursor = Math.max(0, Math.min(index, total - 1));
    const height = Math.max(1, this.gameState.wram.wDexListingHeight);
    let scroll = this.searchResultsScrollOffset;
    if (cursor < scroll) {
      scroll = cursor;
    } else if (cursor >= scroll + height) {
      scroll = cursor - height + 1;
    }
    const maxScroll = Math.max(0, total - height);
    scroll = Math.max(0, Math.min(scroll, maxScroll));
    this.searchResultsCursor = cursor;
    this.searchResultsScrollOffset = scroll;
    this.gameState.wram.wDexListingCursor = cursor;
    this.gameState.wram.wDexListingScrollOffset = scroll;
  }

  private handleAreaAction(): void {
    this.refreshCurrentLocation();
    this.areaOverlay.show();
  }

  private handleCryAction(): void {
    const entry = this.currentEntry;
    if (!entry) {
      return;
    }
    const speciesId = entry.species.id;
    this.ui.playCry?.(speciesId);
  }

  private handlePrintAction(): void {
    const entry = this.currentEntry;
    if (!entry) {
      return;
    }
    const entryData = this.getEntryData(entry);
    const printOption = this.gameState.sram.options.print_option ?? null;
    this.printer.printDexEntry(entry.species.id, entry.pokedexNumber, entryData, printOption);
    this.redisplayEntryScreen();
  }

  redisplayEntryScreen(): void {
    const entryData = this.currentEntryData;
    if (!entryData) {
      return;
    }
    this.entryPageIndex = Math.min(this.entryPageIndex, entryData.pages.length - 1);
  }
}
