// ASM: engine/menus/start_menu.asm
import { MenuUI } from "./types";
import { POKEMON_WORD, POKE_GLYPH, PK_GLYPH, MN_GLYPH } from "@pokecrystal/assets/content/text-constants";
import { HMenuReturn } from "../../core/enums/ui-enums";
import { GameState } from "../../core/state";
import { getFilledSlots } from "../../core/models/party";
import { AudioEngine } from "../../engine/systems/audio";
import { GameButton, KeyEvent, isButtonEvent, isKeyDownEvent } from "../../input/buttons";
import { gameEngine } from "../game-engine";
import { renderFontText } from "../text/render-font";

export class StartMenuResult {
  constructor(
    public readonly action: string,
    public readonly hmenureturn: HMenuReturn,
    public readonly queuedScript: string | null = null,
  ) {}
}

type EntryPrototype = {
  identifier: string;
  label: string;
  description: [string, string];
  handlerFactory: (menu: StartMenu) => () => StartMenuResult;
};

type EntryDefinition = {
  identifier: string;
  label: string;
  description: [string, string];
  handler: () => StartMenuResult;
};

export class ContestStatusDisplay {
  constructor(
    public readonly caughtLabel: string,
    public readonly caughtValue: string,
    public readonly levelLabel: string | null,
    public readonly levelValue: string | null,
    public readonly ballsLabel: string,
    public readonly ballsValue: string,
  ) {}
}

export type StartMenuScriptRunner = {
  registerMenuReturn?: (returnType: HMenuReturn, script?: string | null) => void;
  register_menu_return?: (returnType: HMenuReturn, script?: string | null) => void;
  defer?: (script: string) => void;
  menu_return?: HMenuReturn;
  queued_menu_script?: string | null;
};

export class StartMenu {
  private static readonly MENU_LEFT_TILE = 9;
  private static readonly MENU_TOP_TILE = 0;
  private static readonly MENU_TOP_TILE_CONTEST = 2;
  private static readonly MENU_RIGHT_TILE = Math.floor(160 / 8) - 1;
  private static readonly MENU_MIN_HEIGHT_TILES = 3;
  private static readonly POINTER = "\u25b6";
  private static readonly WINDOW_FILL: [number, number, number] = [255, 255, 255];

  private static readonly MENU_ACCOUNT_LEFT_TILE = 0;
  private static readonly MENU_ACCOUNT_TOP_TILE = 13;
  private static readonly MENU_ACCOUNT_WIDTH_TILES = 12;
  private static readonly MENU_ACCOUNT_HEIGHT_TILES = 5;

  private static readonly CONTEST_BOX_LEFT_TILE = 0;
  private static readonly CONTEST_BOX_TOP_TILE = 0;
  private static readonly CONTEST_BOX_WIDTH_TILES = 17;
  private static readonly CONTEST_BOX_HEIGHT_TILES = 6;

  private static readonly ENTRY_PROTOTYPES: EntryPrototype[] = [
    {
      identifier: "pokedex",
      label: `${POKE_GLYPH}DEX`,
      description: [POKEMON_WORD, "database"],
      handlerFactory: (menu) => menu._handlePokedex.bind(menu),
    },
    {
      identifier: "pokemon",
      label: POKEMON_WORD,
      description: [`Party ${PK_GLYPH}${MN_GLYPH}`, "status"],
      handlerFactory: (menu) => menu._handlePokemon.bind(menu),
    },
    {
      identifier: "pack",
      label: "PACK",
      description: ["Contains", "items"],
      handlerFactory: (menu) => menu._handleBag.bind(menu),
    },
    {
      identifier: "status",
      label: "<PLAYER>",
      description: ["Your own", "status"],
      handlerFactory: (menu) => menu._handleStatus.bind(menu),
    },
    {
      identifier: "save",
      label: "SAVE",
      description: ["Save your", "progress"],
      handlerFactory: (menu) => menu._handleSave.bind(menu),
    },
    {
      identifier: "option",
      label: "OPTION",
      description: ["Change", "settings"],
      handlerFactory: (menu) => menu._handleOption.bind(menu),
    },
    {
      identifier: "exit",
      label: "EXIT",
      description: ["Close this", "menu"],
      handlerFactory: (menu) => menu._handleExit.bind(menu),
    },
    {
      identifier: "pokegear",
      label: `${POKE_GLYPH}GEAR`,
      description: ["Trainer's", "key device"],
      handlerFactory: (menu) => menu._handlePokegear.bind(menu),
    },
    {
      identifier: "quit",
      label: "QUIT",
      description: ["Quit and", "be judged."],
      handlerFactory: (menu) => menu._handleQuit.bind(menu),
    },
  ];

  private readonly entryIndex: Record<string, EntryPrototype> = Object.fromEntries(
    StartMenu.ENTRY_PROTOTYPES.map((entry) => [entry.identifier, entry]),
  );

  private entries: EntryDefinition[] = [];
  private cursorIndexValue = 0;
  private menuAccountEnabled = false;

  constructor(
    private readonly ui: MenuUI,
    private readonly gameState: GameState,
    private readonly audioEngine?: AudioEngine | null,
    private readonly scriptRunner?: StartMenuScriptRunner | null,
  ) {
    this.refresh();
    this.playOpenSound();
  }

  get menuOptions(): string[] {
    return this.entries.map((entry) => entry.label);
  }

  get cursorIndex(): number {
    return this.cursorIndexValue;
  }

  windowBoundsTiles(): [number, number, number, number] {
    const topTile = this.contestActive()
      ? StartMenu.MENU_TOP_TILE_CONTEST
      : StartMenu.MENU_TOP_TILE;
    const widthTiles = StartMenu.MENU_RIGHT_TILE - StartMenu.MENU_LEFT_TILE + 1;
    const heightTiles = Math.max(StartMenu.MENU_MIN_HEIGHT_TILES, this.entries.length + 2);
    return [StartMenu.MENU_LEFT_TILE, topTile, widthTiles, heightTiles];
  }

  menuAccountBoundsTiles(): [number, number, number, number] {
    return [
      StartMenu.MENU_ACCOUNT_LEFT_TILE,
      StartMenu.MENU_ACCOUNT_TOP_TILE,
      StartMenu.MENU_ACCOUNT_WIDTH_TILES,
      StartMenu.MENU_ACCOUNT_HEIGHT_TILES,
    ];
  }

  refresh(): void {
    let activeIdentifier: string | null = null;
    if (this.entries.length && this.cursorIndexValue >= 0 && this.cursorIndexValue < this.entries.length) {
      activeIdentifier = this.entries[this.cursorIndexValue].identifier;
    }

    const contestActive = this.contestActive();
    const entries: EntryDefinition[] = [];
    for (const identifier of this.orderedIdentifiers(contestActive)) {
      entries.push(this.buildEntry(identifier));
    }
    if (entries.length === 0) {
      throw new Error("StartMenu produced no entries; ASM always appends STATUS, SAVE/QUIT, OPTION, and EXIT");
    }

    this.entries = entries;
    const storedIndex = this.gameState.wram.start_menu_cursor;
    if (entries.length > 0) {
      // The menu can gain or lose entries between drawing a frame and handling
      // its input (for example, when a script enables the Pokedex). Keep the
      // entry the player can see selected; its old numeric index may now refer
      // to a different action such as the Pokedex instead of PACK.
      if (activeIdentifier) {
        this.cursorIndexValue = this.findIdentifier(activeIdentifier);
      } else if (storedIndex >= 0 && storedIndex < entries.length) {
        this.cursorIndexValue = storedIndex;
      } else {
        this.cursorIndexValue = 0;
      }
    }

    this.menuAccountEnabled = Boolean(this.gameState.sram.options.menu_account);
  }

  resetCursorPosition(): void {
    this.cursorIndexValue = 0;
    this.gameState.wram.start_menu_cursor = 0;
    this.entries = [];
    this.refresh();
    this.playOpenSound();
  }

  handleInput(event: KeyEvent): StartMenuResult | null {
    if (!isKeyDownEvent(event) || this.entries.length === 0) {
      return null;
    }
    if (isButtonEvent(event, GameButton.B)) {
      this.registerMenuReturn(HMenuReturn.EXIT);
      return new StartMenuResult("close_menu", HMenuReturn.EXIT);
    }
    if (event.key === gameEngine.K_UP) {
      this.cursorIndexValue = (this.cursorIndexValue - 1 + this.entries.length) % this.entries.length;
      this.storeCursor();
      return null;
    }
    if (event.key === gameEngine.K_DOWN) {
      this.cursorIndexValue = (this.cursorIndexValue + 1) % this.entries.length;
      this.storeCursor();
      return null;
    }
    if (isButtonEvent(event, GameButton.A) || isButtonEvent(event, GameButton.Start)) {
      const entry = this.entries[this.cursorIndexValue];
      this.playSound("menu_option");
      const result = entry.handler();
      this.storeCursor();
      return result;
    }
    return null;
  }

  draw(): void {
    const screen = this.ui.screen;
    if (!screen) {
      return;
    }
    const tile = this.ui.tileSize;
    const contestActive = this.contestActive();
    const topTile = contestActive ? StartMenu.MENU_TOP_TILE_CONTEST : StartMenu.MENU_TOP_TILE;
    const baseX = StartMenu.MENU_LEFT_TILE * tile;
    const baseY = topTile * tile;
    const menuHeight = Math.max(StartMenu.MENU_MIN_HEIGHT_TILES, this.entries.length + 2);
    const menuWidth = StartMenu.MENU_RIGHT_TILE - StartMenu.MENU_LEFT_TILE + 1;

    this.ui.drawWindow(screen, baseX, baseY, menuWidth, menuHeight, { fill: StartMenu.WINDOW_FILL });

    this.entries.forEach((entry, index) => {
      const cursorX = baseX + tile;
      const cursorY = baseY + (index + 1) * tile;
      const labelX = baseX + 2 * tile;
      if (index === this.cursorIndexValue) {
        renderFontText(this.ui.font, StartMenu.POINTER, cursorX, cursorY, screen);
      }
      renderFontText(this.ui.font, entry.label, labelX, cursorY, screen);
    });

    if (this.menuAccountEnabled) {
      this.drawMenuAccount();
    }
    if (contestActive) {
      this.drawContestStatus();
    }
  }

  menuAccountLines(): string[] {
    if (!this.menuAccountEnabled || this.entries.length === 0) {
      return [];
    }
    return Array.from(this.entries[this.cursorIndexValue].description);
  }

  contestStatusLines(): string[] {
    const display = this.contestStatusDisplay();
    if (!display) {
      return [];
    }
    const rows: string[] = [`${display.caughtLabel} ${display.caughtValue}`.trimEnd()];
    if (display.levelLabel && display.levelValue) {
      rows.push(`${display.levelLabel} ${display.levelValue}`.trimEnd());
    }
    rows.push(`${display.ballsLabel} ${display.ballsValue}`.trimEnd());
    return rows;
  }

  private orderedIdentifiers(contestActive: boolean): string[] {
    const flags = this.gameState.wram.engine_flags;
    const partyCount = this.gameState.sram.party.pokemon.filter(Boolean).length;
    const hasPokedex = Boolean(flags["ENGINE_POKEDEX"]) || this.gameState.sram.johto_pokedex;
    const hasPokegear = Boolean(flags["ENGINE_POKEGEAR"]);
    const order: string[] = [];
    if (hasPokedex) order.push("pokedex");
    if (partyCount > 0) order.push("pokemon");
    if (!contestActive) order.push("pack");
    if (hasPokegear) order.push("pokegear");
    order.push("status");
    order.push(contestActive ? "quit" : "save");
    order.push("option");
    order.push("exit");
    return order;
  }

  private buildEntry(identifier: string): EntryDefinition {
    const prototype = this.entryIndex[identifier];
    if (!prototype) {
      throw new Error(`StartMenu requested unsupported ASM entry '${identifier}'`);
    }
    const label = this.resolveLabel(prototype.label);
    const handler = prototype.handlerFactory(this);
    return { identifier, label, description: prototype.description, handler };
  }

  private findIdentifier(identifier: string): number {
    const index = this.entries.findIndex((entry) => entry.identifier === identifier);
    return index >= 0 ? index : 0;
  }

  private drawMenuAccount(): void {
    const screen = this.ui.screen;
    if (!screen) {
      return;
    }
    const tile = this.ui.tileSize;
    const boxX = StartMenu.MENU_ACCOUNT_LEFT_TILE * tile;
    const boxY = StartMenu.MENU_ACCOUNT_TOP_TILE * tile;
    this.ui.drawWindow(screen, boxX, boxY, StartMenu.MENU_ACCOUNT_WIDTH_TILES, StartMenu.MENU_ACCOUNT_HEIGHT_TILES, {
      fill: StartMenu.WINDOW_FILL,
    });
    const startYTile = StartMenu.MENU_ACCOUNT_TOP_TILE + 1;
    this.menuAccountLines().forEach((line, row) => {
      const textX = (StartMenu.MENU_ACCOUNT_LEFT_TILE + 1) * tile;
      const textY = (startYTile + row) * tile;
      renderFontText(this.ui.font, line, textX, textY, screen);
    });
  }

  private drawContestStatus(): void {
    const display = this.contestStatusDisplay();
    if (!display) {
      return;
    }
    const screen = this.ui.screen;
    if (!screen) {
      return;
    }
    const tile = this.ui.tileSize;
    const boxX = StartMenu.CONTEST_BOX_LEFT_TILE * tile;
    const boxY = StartMenu.CONTEST_BOX_TOP_TILE * tile;
    this.ui.drawWindow(screen, boxX, boxY, StartMenu.CONTEST_BOX_WIDTH_TILES, StartMenu.CONTEST_BOX_HEIGHT_TILES, {
      fill: StartMenu.WINDOW_FILL,
    });
    this.renderText(display.caughtLabel, 1, 1);
    this.renderText(display.caughtValue, 8, 1);
    if (display.levelLabel && display.levelValue) {
      this.renderText(display.levelLabel, 1, 3);
      this.renderText(display.levelValue, 8, 3);
    }
    this.renderText(display.ballsLabel, 1, 5);
    this.renderText(display.ballsValue, 8, 5);
  }

  private _handlePokedex(): StartMenuResult {
    this.registerMenuReturn(HMenuReturn.REOPEN);
    return new StartMenuResult("pokedex", HMenuReturn.REOPEN);
  }

  private _handlePokemon(): StartMenuResult {
    this.registerMenuReturn(HMenuReturn.REOPEN);
    return new StartMenuResult("pokemon_menu", HMenuReturn.REOPEN);
  }

  private _handleBag(): StartMenuResult {
    this.registerMenuReturn(HMenuReturn.REOPEN);
    return new StartMenuResult("bag_menu", HMenuReturn.REOPEN);
  }

  private _handlePokegear(): StartMenuResult {
    this.registerMenuReturn(HMenuReturn.REOPEN);
    return new StartMenuResult("pokegear", HMenuReturn.REOPEN);
  }

  private _handleStatus(): StartMenuResult {
    this.registerMenuReturn(HMenuReturn.REOPEN);
    return new StartMenuResult("trainer_card", HMenuReturn.REOPEN);
  }

  private _handleSave(): StartMenuResult {
    this.registerMenuReturn(HMenuReturn.EXIT);
    return new StartMenuResult("save", HMenuReturn.EXIT);
  }

  private _handleOption(): StartMenuResult {
    this.registerMenuReturn(HMenuReturn.REDRAW);
    return new StartMenuResult("options_menu", HMenuReturn.REDRAW);
  }

  private _handleExit(): StartMenuResult {
    this.registerMenuReturn(HMenuReturn.EXIT);
    return new StartMenuResult("close_menu", HMenuReturn.EXIT);
  }

  private _handleQuit(): StartMenuResult {
    const script = "BugCatchingContestReturnToGateScript";
    this.registerMenuReturn(HMenuReturn.SCRIPT, script);
    return new StartMenuResult("overworld", HMenuReturn.SCRIPT, script);
  }

  private contestActive(): boolean {
    const state = this.gameState.wram.bug_contest_state;
    const flags = this.gameState.wram.engine_flags;
    const active = Boolean(state.timer_active) || Boolean(flags["ENGINE_BUG_CONTEST_TIMER"]);
    if (active && !state.timer_active) {
      state.timer_active = true;
    }
    return active;
  }

  private contestStatusDisplay(): ContestStatusDisplay | null {
    if (!this.contestActive()) {
      return null;
    }
    const state = this.gameState.wram.bug_contest_state;
    const species = state.caught_species;
    const caughtValue = species ? String(species).toUpperCase() : "None";
    let levelLabel: string | null = null;
    let levelValue: string | null = null;
    if (species) {
      levelLabel = "LEVEL";
      levelValue = String(state.caught_level ?? 0);
    }
    const balls = Math.max(0, state.park_balls_remaining ?? 0);
    return new ContestStatusDisplay("CAUGHT", caughtValue, levelLabel, levelValue, "BALLS:", String(balls));
  }

  private playSound(name: string): void {
    this.audioEngine?.playSound(name);
  }

  private playOpenSound(): void {
    this.audioEngine?.playSound("SFX_MENU");
  }

  private storeCursor(): void {
    this.gameState.wram.start_menu_cursor = this.cursorIndexValue;
  }

  private registerMenuReturn(returnType: HMenuReturn, script?: string | null): void {
    if (!this.scriptRunner) {
      return;
    }
    if (typeof this.scriptRunner.registerMenuReturn === "function") {
      this.scriptRunner.registerMenuReturn(returnType, script ?? null);
      return;
    }
    if (typeof this.scriptRunner.register_menu_return === "function") {
      this.scriptRunner.register_menu_return(returnType, script ?? null);
      return;
    }
    this.scriptRunner.menu_return = returnType;
    this.scriptRunner.queued_menu_script = script ?? null;
    if (returnType === HMenuReturn.SCRIPT && script && typeof this.scriptRunner.defer === "function") {
      this.scriptRunner.defer(script);
    }
  }

  private resolveLabel(template: string): string {
    if (template === "<PLAYER>") {
      return this.playerLabel();
    }
    return template;
  }

  private playerLabel(): string {
    const name = (this.gameState.sram.player_name || "").trim();
    return name || "?????";
  }

  private renderText(text: string, tileX: number, tileY: number): void {
    const screen = this.ui.screen;
    if (!screen || !text) {
      return;
    }
    const tile = this.ui.tileSize;
    renderFontText(this.ui.font, text, tileX * tile, tileY * tile, screen);
  }
}
