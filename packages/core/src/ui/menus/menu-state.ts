// ASM: engine/menus/menu.asm start menu and TM/HM flow coordinator.
import { MenuUI } from "./types";
import { asmTextLoader } from "../../core/asm-text-loader";
import { GameState } from "../../core/state";
import { SaveGameError } from "../../core/save";
import logger from "../../core/logger";
import { AudioEngine } from "../../engine/systems/audio";
import { ItemSystem } from "../../engine/systems/items";
import { syncGameClock } from "../../engine/systems/time";
import { DefaultPrinter, Printer } from "../../engine/systems/printer";
import { MoveName } from "../../core/enums/move";
import { MonMenuItem } from "../../core/enums/mon-menu";
import { PartyMenuAction } from "../../core/enums/party-menu";
import { ItemPocket } from "../../core/enums/item";
import { Item, Move, Pokemon, type PokemonSpecies } from "../../core/models";
import { countPokedexEntries } from "../../core/pokedex";
import { DataLoader } from "../../core/data-loader";
import type { ScriptRunner } from "../../engine/world/story-events/runner";
import { TextFormatter } from "../../engine/world/story-events/text-formatter";
import { gameEngine } from "../game-engine";
import { KeyEvent, isCancelEvent, isConfirmEvent, isKeyDownEvent } from "../../input/buttons";
import { BagMenu } from "./bag-menu";
import { Menu } from "./menu";
import { StartMenu, type StartMenuScriptRunner } from "./start-menu";
import { PokemonMenu } from "./pokemon-menu";
import { PokemonStatsScreen } from "./pokemon-stats";
import { PokedexScreen } from "./pokedex";
import { PokegearScreen } from "./pokegear";
import { OptionsMenu } from "./options-menu";
import { TrainerCardScreen } from "./trainer-card";
import { MoveReorderMenu } from "./move-reorder-menu";
import {
  buildActiveOptionMenuControlLines,
  buildActiveOptionMenuLines,
  buildBagControlLines,
  buildBagMenuLines,
  buildDialogueControlLines,
  buildMoveMenuControlLines,
  buildMoveMenuLines,
  buildOptionsMenuControlLines,
  buildOptionsMenuLines,
  buildPokemonMenuControlLines,
  buildPokemonMenuLines,
  buildPokemonStatsControlLines,
  buildPromptControlLines,
  buildStartMenuControlLines,
  buildTrainerCardControlLines,
  renderTextSnapshot,
} from "../text-overlays";
import { DialogueWindow, YesNoPrompt } from "../text/dialogue";
import { isTextUI } from "../screens/screen-types";
import * as tmhmSystem from "../../engine/systems/tmhm";
import { EvolutionItemHandler } from "./evolution-item-handler";
import { FieldItemHandler } from "./field-item-handler";
import { nextFrame } from "../async-loop";

type MenuDataLoader = DataLoader & {
  ensureMenuData?: () => void;
  ensure_menu_data?: () => void;
  load_move_data?: () => void;
  move_data?: Record<string, Move>;
  pokemonData?: Map<string, PokemonSpecies> | Record<string, PokemonSpecies>;
  pokemon_data?: Record<string, PokemonSpecies>;
  itemData?: Map<string, Item> | Record<string, Item>;
  item_data?: Record<string, Item>;
  getText?: (label: string) => string | null;
  get_text?: (label: string) => string | null;
  speciesMap?: Map<string, PokemonSpecies>;
  getPokemonSpecies?: (id: string) => PokemonSpecies | null;
  get_pokemon_species?: (id: string) => PokemonSpecies | null;
  getSpecies?: (id: string) => PokemonSpecies | null;
};

type MenuScriptRunner = ScriptRunner & StartMenuScriptRunner;
type PokedexScreenOptions = NonNullable<ConstructorParameters<typeof PokedexScreen>[2]>;

enum TMHMStage {
  INTRO_BOOT = "INTRO_BOOT",
  INTRO_MOVE = "INTRO_MOVE",
  CONFIRM_USE = "CONFIRM_USE",
  CHOOSE_POKEMON = "CHOOSE_POKEMON",
  MESSAGE = "MESSAGE",
  FORGET_PROMPT = "FORGET_PROMPT",
  CONFIRM_STOP_PROMPT = "CONFIRM_STOP_PROMPT",
  FORGET_MENU = "FORGET_MENU",
  COMPLETE = "COMPLETE",
}

type TMHMUseContext = {
  itemName: string;
  move: MoveName;
  isHm: boolean;
  stage: TMHMStage;
  nextStage: TMHMStage | null;
  target: Pokemon | null;
  pendingCompletion: boolean | null;
  forgetOptions: string[];
};

const REPEL_DURATIONS: Record<string, number> = {
  REPEL: 100,
  SUPER_REPEL: 200,
  MAX_REPEL: 250,
};

const EVOLUTION_STONES = new Set([
  "FIRE_STONE",
  "WATER_STONE",
  "THUNDERSTONE",
  "LEAF_STONE",
  "SUN_STONE",
  "MOON_STONE",
]);

export class MenuState {
  private readonly screen;
  public readonly itemSystem: ItemSystem;
  public readonly dialogue: DialogueWindow;
  private dialogueVisible = false;
  private dialogueAfterClose: Array<() => void> = [];
  private activeOptionMenu: Menu | null = null;
  private tmhmContext: TMHMUseContext | null = null;
  private tmhmPromptScript: boolean[] | null = null;
  private waitingForBagItemSelection = false;
  private tmhmYesNoPrompt:
    | {
        prompt: YesNoPrompt;
        onResult: (confirmed: boolean) => void;
      }
    | null = null;
  private evoHandler: EvolutionItemHandler;
  private fieldItemHandler: FieldItemHandler;
  private printer: Printer;

  private closeMenuRequested = false;
  public currentMenu = "start_menu";

  private startMenu!: StartMenu;
  private pokemonStats!: PokemonStatsScreen;
  public pokemonMenu: PokemonMenu | null = null;
  private moveMenu!: MoveReorderMenu;
  private bagMenu: BagMenu | null = null;
  private pokedex: PokedexScreen | null = null;
  private pokegear!: PokegearScreen;
  private optionsMenu!: OptionsMenu;
  private trainerCard!: TrainerCardScreen;
  private saveFlow:
    | {
        stage: "prompt" | "saving" | "saved";
        prompt: YesNoPrompt | null;
        saveCallback: () => boolean | Promise<boolean>;
        saved: boolean;
      }
    | null = null;

  constructor(
    private readonly ui: MenuUI,
    public readonly gameState: GameState,
    private readonly audioEngine: AudioEngine,
    _tileset: unknown,
    public readonly dataLoader?: MenuDataLoader | null,
    private readonly scriptRunner?: MenuScriptRunner | null,
    private readonly overworld?: {
      use_hm_from_menu?: (moveName: string, pokemon: Pokemon) => Promise<boolean> | boolean;
      handle_dig?: (pokemon: Pokemon) => Promise<boolean> | boolean | void;
      handle_headbutt?: (pokemon: Pokemon) => Promise<boolean> | boolean;
      handle_sweet_scent?: (pokemon: Pokemon) => Promise<boolean> | boolean;
      handle_teleport?: () => Promise<boolean> | boolean;
      handle_rock_smash?: (pokemon: Pokemon) => Promise<boolean> | boolean;
      use_key_item?: (item: string) => Promise<boolean> | boolean;
      _show_field_move_text?: (label: string) => void;
      _show_field_move_text_async?: (label: string) => Promise<void>;
    },
    printer?: Printer | null,
  ) {
    this.screen = ui.screen;
    this.itemSystem = new ItemSystem(gameState, dataLoader ?? undefined);
    this.dialogue = new DialogueWindow(ui, gameState, 2, audioEngine);
    this.evoHandler = new EvolutionItemHandler(this);
    this.fieldItemHandler = new FieldItemHandler(this);
    this.printer = printer ?? new DefaultPrinter();
    this.reset();
  }

  reset(): void {
    this.gameState.wram.start_menu_cursor = 0;
    this.currentMenu = "start_menu";
    this.closeMenuRequested = false;
    this.dialogueAfterClose = [];
    this.evoHandler.reset();
    this.fieldItemHandler.reset();
    this.startMenu = new StartMenu(this.ui, this.gameState, this.audioEngine, this.scriptRunner ?? null);
    this.pokemonStats = new PokemonStatsScreen(this.ui, this.gameState);
    this.pokemonMenu = null;
    this.moveMenu = new MoveReorderMenu(this.ui, this.gameState, this.audioEngine);
    this.tmhmPromptScript = null;
    this.tmhmYesNoPrompt = null;
    this.bagMenu = null;
    this.pokedex = null;
    this.saveFlow = null;
    this.pokegear = new PokegearScreen(this.ui, this.gameState, {
      scriptRunner: this.scriptRunner ?? null,
      audioEngine: this.audioEngine,
    });
    this.optionsMenu = new OptionsMenu(this.ui, this.audioEngine, this.gameState);
    this.trainerCard = new TrainerCardScreen(this.ui, this.gameState);
  }

  private ensureMenuAssets(): void {
    this.dataLoader?.ensureMenuData?.();
  }

  private ensureBagMenu(): BagMenu {
    this.ensureMenuAssets();
    if (!this.bagMenu) {
      this.bagMenu = new BagMenu(
        this.ui,
        this.gameState,
        this.audioEngine,
        this.dataLoader ?? undefined,
      );
    }
    return this.bagMenu;
  }

  private ensurePokedex(): PokedexScreen {
    this.ensureMenuAssets();
    if (!this.pokedex) {
      const options: PokedexScreenOptions = {
        dataLoader: (this.dataLoader ?? null) as NonNullable<PokedexScreenOptions>["dataLoader"],
        audioEngine: this.audioEngine ?? null,
        scriptRunner: this.scriptRunner ?? null,
        printer: this.printer,
      };
      this.pokedex = new PokedexScreen(this.ui, this.gameState, options);
    }
    return this.pokedex;
  }

  private ensurePokemonMenu(): PokemonMenu {
    if (!this.pokemonMenu) {
      this.pokemonMenu = new PokemonMenu(this.ui, this.gameState, this.audioEngine);
      this.pokemonMenu.setItemSelector((pokemon) => this.promptItemSelectionFromBag(pokemon));
      this.pokemonMenu.registerMonMenuHandler(MonMenuItem.STATS, (_menu, pokemon) => this.openPokemonStats(pokemon));
      this.pokemonMenu.registerMonMenuHandler(MonMenuItem.MOVE, (_menu, pokemon) => this.openMoveMenu(pokemon));
      if (this.overworld) {
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.CUT, (_menu, pokemon) => this.useCut(pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.FLY, (_menu, pokemon) => this.useHm(MoveName.FLY, pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.SURF, (_menu, pokemon) => this.useHm(MoveName.SURF, pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.STRENGTH, (_menu, pokemon) => this.useHm(MoveName.STRENGTH, pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.FLASH, (_menu, pokemon) => this.useHm(MoveName.FLASH, pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.WATERFALL, (_menu, pokemon) => this.useHm(MoveName.WATERFALL, pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.WHIRLPOOL, (_menu, pokemon) => this.useHm(MoveName.WHIRLPOOL, pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.DIG, (_menu, pokemon) => this.useDig(pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.HEADBUTT, (_menu, pokemon) => this.useHeadbutt(pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.ROCKSMASH, (_menu, pokemon) => this.useRockSmash(pokemon));
        this.pokemonMenu.registerMonMenuHandler(MonMenuItem.SWEETSCENT, (_menu, pokemon) => this.useSweetScent(pokemon));
      }
      this.pokemonMenu.registerMonMenuHandler(MonMenuItem.TELEPORT, (_menu, pokemon) => this.useTeleport(pokemon));
    }
    return this.pokemonMenu;
  }

  handleInput(event: KeyEvent): string | null {
    if (this.waitingForBagItemSelection) {
      return null;
    }
    if (this.handleModalInput(event)) {
      return null;
    }
    if (this.currentMenu === "start_menu") {
      this.startMenu.refresh();
      const result = this.startMenu.handleInput(event);
      if (!result) {
        return null;
      }
      if (result.action === "close_menu") {
        return "close_menu";
      }
      if (result.action === "exit") {
        return "exit";
      }
      if (result.action === "save") {
        return "save";
      }
      if (result.action === "overworld") {
        return "overworld";
      }
      if (
        [
          "pokedex",
          "pokemon_menu",
          "bag_menu",
          "pokegear",
          "options_menu",
          "trainer_card",
        ].includes(result.action)
      ) {
        if (result.action === "pokemon_menu") {
          this.ensurePokemonMenu().reset();
        }
        if (result.action === "pokedex") {
          this.ensurePokedex().reset();
        }
        if (result.action === "options_menu") {
          this.optionsMenu.reset();
        }
        if (result.action === "trainer_card") {
          this.trainerCard.reset();
        }
        if (result.action === "bag_menu") {
          this.ensureBagMenu();
        }
        this.currentMenu = result.action;
        return result.action;
      }
      return result.action;
    }
    if (this.currentMenu === "pokemon_menu") {
      if (isKeyDownEvent(event)) {
        const menuEvent = {
          ...event,
          type: typeof event.type === "string" ? event.type : String(event.type ?? ""),
        };
        const selection = this.ensurePokemonMenu().handleInput(menuEvent);
        if (selection && this.currentMenu === "pokemon_menu") {
          const [action] = selection;
          if (action === "cancel") {
            this.returnToStartMenu();
          }
        }
      }
    } else if (this.currentMenu === "pokemon_stats") {
      const result = this.pokemonStats.handleInput(event);
      if (result === "exit") {
        this.ensurePokemonMenu().reset({ preserveWramCursor: true });
        this.currentMenu = "pokemon_menu";
      }
    } else if (this.currentMenu === "move_menu") {
      const result = this.moveMenu.handleInput(event);
      if (result === "exit") {
        this.currentMenu = "pokemon_menu";
      }
    } else if (this.currentMenu === "bag_menu") {
      const selection = this.ensureBagMenu().handleInput(event);
      if (selection) {
        const [action] = selection;
        if (action === "cancel") {
          this.returnToStartMenu();
        } else {
          this.handleBagSelection(selection);
        }
      }
    } else if (this.currentMenu === "pokedex") {
      const result = this.ensurePokedex().handleInput(event);
      if (result === "exit") {
        this.returnToStartMenu();
      }
    } else if (this.currentMenu === "pokegear") {
      const result = this.pokegear.handleInput(event);
      if (result === "exit") {
        this.returnToStartMenu();
      }
    } else if (this.currentMenu === "options_menu") {
      const result = this.optionsMenu.handleInput(event);
      if (result === "exit") {
        this.returnToStartMenu();
        this.optionsMenu.reset();
      }
    } else if (this.currentMenu === "trainer_card") {
      const result = this.trainerCard.handleInput(event);
      if (result === "exit") {
        this.returnToStartMenu();
      }
    }
    if (this.consumeCloseRequestInternal()) {
      return "close_menu";
    }
    return null;
  }

  draw(): void {
    syncGameClock(this.gameState);
    this.dialogue.update();
    const textOnlyUi = this.isTextOnlyUi();
    if (textOnlyUi) {
      if (this.currentMenu === "move_menu") {
        this.moveMenu.update();
      } else if (this.currentMenu === "options_menu") {
        this.optionsMenu.update();
      }
      if (this.activeOptionMenu) {
        this.activeOptionMenu.update();
      }
      this.renderTextOverlay();
      return;
    }
    if (this.currentMenu === "start_menu") {
      this.startMenu.draw();
    } else if (this.currentMenu === "pokemon_menu") {
      this.ensurePokemonMenu().draw();
    } else if (this.currentMenu === "pokemon_stats") {
      this.pokemonStats.draw();
    } else if (this.currentMenu === "move_menu") {
      this.moveMenu.update();
      this.moveMenu.draw();
    } else if (this.currentMenu === "bag_menu") {
      const bagMenu = this.ensureBagMenu();
      bagMenu.draw();
    } else if (this.currentMenu === "pokedex") {
      this.ensurePokedex().draw();
    } else if (this.currentMenu === "pokegear") {
      this.pokegear.draw();
    } else if (this.currentMenu === "options_menu") {
      this.optionsMenu.update();
      this.optionsMenu.draw();
    } else if (this.currentMenu === "trainer_card") {
      this.trainerCard.draw();
    }
    if (this.activeOptionMenu) {
      this.activeOptionMenu.update();
      this.activeOptionMenu.draw();
    }
    if (this.dialogueVisible) {
      this.dialogue.draw();
    }
    const activePrompt = this.activeYesNoPrompt();
    if (activePrompt) {
      activePrompt.draw();
    }
    this.renderTextOverlay();
  }

  private activeYesNoPrompt(): YesNoPrompt | null {
    return this.saveFlow?.prompt ?? this.tmhmYesNoPrompt?.prompt ?? null;
  }

  private renderTextOverlay(): void {
    const dialogueLines = this.buildDialogueLines();
    const promptLines = this.buildPromptLines();
    const controlLines = this.resolveControlLines();
    if (this.currentMenu === "start_menu") {
      const menuLines = this.startMenu.menuOptions.map((label, idx) =>
        this.formatCursorLine(label, idx === this.startMenu.cursorIndex)
      );
      const viewportLines = ["START MENU"];
      const accountLines = this.startMenu.menuAccountLines();
      if (accountLines.length) {
        viewportLines.push("MENU ACCOUNT:");
        viewportLines.push(...accountLines);
      }
      const contestLines = this.startMenu.contestStatusLines();
      if (contestLines.length) {
        viewportLines.push("CONTEST:");
        viewportLines.push(...contestLines);
      }
      renderTextSnapshot(this.ui, {
        viewportLines,
        infoLines: controlLines,
        viewportTitle: "Start Menu",
        infoTitle: "Legend",
        menuLines,
        promptLines,
        dialogueLines,
      });
      return;
    }
    if (this.currentMenu === "bag_menu") {
      const bagMenu = this.ensureBagMenu();
      const money = this.gameState.sram?.money ?? 0;
      const viewportLines = ["BAG", `MONEY: ¥${Number(money).toLocaleString("en-US")}`];
      renderTextSnapshot(this.ui, {
        viewportLines,
        infoLines: controlLines,
        viewportTitle: "Bag",
        infoTitle: "Legend",
        menuLines: buildBagMenuLines(bagMenu),
        promptLines,
        dialogueLines,
      });
      return;
    }
    if (this.currentMenu === "pokemon_menu") {
      const pokemonMenu = this.ensurePokemonMenu();
      const viewportLines = ["POKEMON MENU"];
      const switchOrigin = pokemonMenu.getSwitchOrigin();
      if (pokemonMenu.getMode() === "switch" && switchOrigin !== null) {
        const entries = pokemonMenu.getPartyEntries();
        const originEntry = entries[switchOrigin];
        if (originEntry) {
          viewportLines.push(`SWAP FROM: ${this.formatPokemonLabel(originEntry.pokemon)}`);
        }
      }
      const menuLines = buildPokemonMenuLines(pokemonMenu);
      if (this.activeOptionMenu) {
        menuLines.push("OPTIONS:");
        menuLines.push(...buildActiveOptionMenuLines(this.activeOptionMenu));
      }
      renderTextSnapshot(this.ui, {
        viewportLines,
        infoLines: controlLines,
        viewportTitle: "Pokemon Menu",
        infoTitle: "Legend",
        menuLines,
        promptLines,
        dialogueLines,
      });
      return;
    }
    if (this.currentMenu === "pokemon_stats") {
      const overlay = this.pokemonStats.getTextOverlay();
      renderTextSnapshot(this.ui, {
        viewportLines: overlay.viewportLines,
        infoLines: controlLines,
        viewportTitle: "Pokemon Stats",
        infoTitle: "Legend",
        menuLines: overlay.menuLines,
        promptLines,
        dialogueLines,
      });
      return;
    }
    if (this.currentMenu === "move_menu") {
      const pokemon = this.moveMenu.getActivePokemon();
      const viewportLines = ["MOVE REORDER"];
      if (pokemon) {
        viewportLines.push(`POKEMON: ${this.formatPokemonLabel(pokemon)}`);
      }
      const swapOrigin = this.moveMenu.getSwapOrigin();
      if (swapOrigin !== null) {
        const moves = this.moveMenu.getMoveNames();
        const moveName = moves[swapOrigin] ?? "UNKNOWN";
        viewportLines.push(`SWAP FROM: ${moveName}`);
      }
      renderTextSnapshot(this.ui, {
        viewportLines,
        infoLines: controlLines,
        viewportTitle: "Move Menu",
        infoTitle: "Legend",
        menuLines: buildMoveMenuLines(this.moveMenu),
        promptLines,
        dialogueLines,
      });
      return;
    }
    if (this.currentMenu === "pokedex") {
      const overlay = this.ensurePokedex().getTextOverlay();
      renderTextSnapshot(this.ui, {
        viewportLines: overlay.viewportLines,
        infoLines: controlLines,
        viewportTitle: "Pokedex",
        infoTitle: "Legend",
        menuLines: overlay.menuLines,
        promptLines,
        dialogueLines,
      });
      return;
    }
    if (this.currentMenu === "pokegear") {
      const overlay = this.pokegear.getTextOverlay();
      renderTextSnapshot(this.ui, {
        viewportLines: overlay.viewportLines,
        infoLines: controlLines,
        viewportTitle: "Pokegear",
        infoTitle: "Legend",
        menuLines: overlay.menuLines,
        promptLines,
        dialogueLines,
      });
      return;
    }
    if (this.currentMenu === "options_menu") {
      renderTextSnapshot(this.ui, {
        viewportLines: ["OPTIONS"],
        infoLines: controlLines,
        viewportTitle: "Options",
        infoTitle: "Legend",
        menuLines: buildOptionsMenuLines(this.optionsMenu),
        promptLines,
        dialogueLines,
      });
      return;
    }
    if (this.currentMenu === "trainer_card") {
      const viewportLines = ["TRAINER CARD", ...this.buildTrainerCardInfoLines()];
      renderTextSnapshot(this.ui, {
        viewportLines,
        infoLines: controlLines,
        viewportTitle: "Trainer Card",
        infoTitle: "Legend",
        menuLines: null,
        promptLines,
        dialogueLines,
      });
    }
  }

  private isTextOnlyUi(): boolean {
    return isTextUI(this.ui as Parameters<typeof isTextUI>[0]) &&
      typeof (this.ui as { getChildren?: () => unknown[] }).getChildren !== "function";
  }

  private buildDialogueLines(): string[] | null {
    if (!this.dialogueVisible) {
      return null;
    }
    const text = this.dialogue.visible_text ?? "";
    const lines = String(text)
      .split("\n")
      .map((line) => line.trimEnd())
      .filter((line) => line.length > 0);
    return lines.length ? lines : null;
  }

  private buildPromptLines(): string[] | null {
    const prompt = this.activeYesNoPrompt();
    if (!prompt) {
      return null;
    }
    return ["YES", "NO"].map((label, idx) =>
      this.formatCursorLine(label, idx === prompt.selection)
    );
  }

  private buildTrainerCardInfoLines(): string[] {
    const sram = this.gameState.sram;
    const name = (sram.player_name || "?????").trim() || "?????";
    const trainerId = String(sram.player_id ?? 0).padStart(5, "0");
    const money = Number(sram.money ?? 0);
    const dexOwned = countPokedexEntries(sram.pokedex_owned);
    const hasDex = Boolean(
      sram.johto_pokedex ||
        countPokedexEntries(sram.pokedex_seen) ||
        dexOwned
    );
    const johto = sram.badges.johto.filter(Boolean).length;
    const kanto = sram.badges.kanto.filter(Boolean).length;
    const badgeTotal = johto + kanto;
    const hours = Math.max(0, Math.min(sram.game_time_hours, 9999));
    const minutes = Math.max(0, Math.min(sram.game_time_minutes, 59));
    return [
      `PAGE: ${this.trainerCard.getActivePage()}`,
      `PLAYER: ${name}`,
      `ID: ${trainerId}`,
      `MONEY: ¥${money.toLocaleString("en-US")}`,
      `POKEDEX: ${hasDex ? dexOwned : "---"}`,
      `BADGES: ${badgeTotal}/16 (JOHTO ${johto}/8 KANTO ${kanto}/8)`,
      `TIME: ${String(hours).padStart(2, "0")}:${String(minutes).padStart(2, "0")}`,
    ];
  }

  private resolveControlLines(): string[] {
    if (this.activeYesNoPrompt()) {
      return buildPromptControlLines();
    }
    if (this.dialogueVisible) {
      return buildDialogueControlLines();
    }
    if (this.activeOptionMenu) {
      return buildActiveOptionMenuControlLines();
    }
    if (this.currentMenu === "start_menu") {
      return buildStartMenuControlLines();
    }
    if (this.currentMenu === "bag_menu") {
      return buildBagControlLines(this.ensureBagMenu());
    }
    if (this.currentMenu === "pokemon_menu") {
      return buildPokemonMenuControlLines(this.ensurePokemonMenu());
    }
    if (this.currentMenu === "pokemon_stats") {
      return buildPokemonStatsControlLines(this.pokemonStats.getActivePokemon());
    }
    if (this.currentMenu === "move_menu") {
      return buildMoveMenuControlLines(this.moveMenu);
    }
    if (this.currentMenu === "pokedex") {
      return this.ensurePokedex().getControlLines();
    }
    if (this.currentMenu === "pokegear") {
      return this.pokegear.getControlLines();
    }
    if (this.currentMenu === "options_menu") {
      return buildOptionsMenuControlLines();
    }
    if (this.currentMenu === "trainer_card") {
      return buildTrainerCardControlLines();
    }
    return [];
  }

  private formatCursorLine(label: string, active: boolean): string {
    return `${active ? "\u25b6" : " "} ${label}`;
  }

  private formatPokemonLabel(mon: Pokemon): string {
    const nickname = (mon as Pokemon & { nickname?: string }).nickname ?? "";
    if (nickname) {
      return nickname;
    }
    const speciesId = (mon as Pokemon & { species?: { id?: string } }).species?.id ?? "";
    return speciesId || "UNKNOWN";
  }

  private formatPokemonStatus(mon: Pokemon): string {
    const status = (mon as { status?: unknown }).status;
    if (!status) {
      return "OK";
    }
    if (typeof status === "string") {
      return status || "OK";
    }
    if (typeof status === "object") {
      const name = (status as { name?: string }).name;
      if (name) {
        return String(name);
      }
    }
    return String(status);
  }

  private openPokemonStats(pokemon: Pokemon): void {
    this.pokemonStats.showPokemon(pokemon);
    this.currentMenu = "pokemon_stats";
  }

  private openMoveMenu(pokemon: Pokemon): void {
    this.moveMenu.showPokemon(pokemon);
    this.currentMenu = "move_menu";
  }

  private useCut(pokemon: Pokemon): void {
    this.useHm(MoveName.CUT, pokemon);
  }

  private useHm(moveName: MoveName, pokemon: Pokemon): void {
    const result = this.overworld?.use_hm_from_menu?.(moveName, pokemon);
    this.closeOnFieldMoveStartOrSuccess(result);
  }

  private closeOnFieldMoveStartOrSuccess(result: Promise<boolean> | boolean | void | undefined): void {
    if (result === undefined) {
      return;
    }
    if (result instanceof Promise) {
      this.requestMenuClose();
      result.catch((error) => {
        logger.error("[menu] Field move failed", error);
      });
    } else if (result) {
      this.requestMenuClose();
    }
  }

  private useDig(pokemon: Pokemon): void {
    this.closeOnFieldMoveStartOrSuccess(this.overworld?.handle_dig?.(pokemon));
  }

  private useHeadbutt(pokemon: Pokemon): void {
    this.closeOnFieldMoveStartOrSuccess(this.overworld?.handle_headbutt?.(pokemon));
  }

  private useSweetScent(pokemon: Pokemon): void {
    this.closeOnFieldMoveStartOrSuccess(this.overworld?.handle_sweet_scent?.(pokemon));
  }

  private useRockSmash(pokemon: Pokemon): void {
    this.closeOnFieldMoveStartOrSuccess(this.overworld?.handle_rock_smash?.(pokemon));
  }

  private useTeleport(_pokemon: Pokemon): void {
    this.closeOnFieldMoveStartOrSuccess(this.overworld?.handle_teleport?.());
  }

  private returnToStartMenu(): void {
    this.startMenu.resetCursorPosition();
    this.currentMenu = "start_menu";
  }

  private requestMenuClose(): void {
    this.closeMenuRequested = true;
    this.returnToStartMenu();
  }

  private consumeCloseRequestInternal(): boolean {
    if (!this.closeMenuRequested) {
      return false;
    }
    this.closeMenuRequested = false;
    return true;
  }

  consumeCloseRequest(): boolean {
    return this.consumeCloseRequestInternal();
  }

  openOptionsMenu(): void {
    this.optionsMenu.reset();
    this.currentMenu = "options_menu";
  }

  private handleModalInput(event: KeyEvent): boolean {
    if (this.saveFlow) {
      return this.handleSaveFlowInput(event);
    }
    if (this.tmhmYesNoPrompt) {
      return this.handleTmhmYesNoInput(event);
    }
    if (this.activeOptionMenu) {
      const choice = this.activeOptionMenu.handleInput(event);
      if (choice) {
        this.handleForgetMenuChoice(choice);
      }
      return true;
    }
    if (!this.dialogueVisible) {
      return false;
    }
    if (!isKeyDownEvent(event)) {
      return true;
    }
    if (isConfirmEvent(event) || isCancelEvent(event)) {
      this.dismissDialogue();
      return true;
    }
    return true;
  }

  private handleTmhmYesNoInput(event: KeyEvent): boolean {
    const flow = this.tmhmYesNoPrompt;
    if (!flow) {
      return false;
    }
    this.dialogue.handle_input(event);
    flow.prompt.handle_input(event);
    if (!flow.prompt.finished) {
      return true;
    }
    const confirmed = flow.prompt.result();
    this.tmhmYesNoPrompt = null;
    this.dialogue.complete();
    this.dialogue.update();
    this.dialogueVisible = false;
    flow.onResult(confirmed);
    return true;
  }

  private startTmhmYesNoPrompt(text: string, onResult: (confirmed: boolean) => void): void {
    if (this.tmhmPromptScript) {
      const scripted = this.tmhmPromptScript.length ? Boolean(this.tmhmPromptScript.shift()) : true;
      onResult(scripted);
      return;
    }
    if (!gameEngine.display.get_init()) {
      logger.info("[menu] TM/HM prompt bypassed (display not initialized)");
      onResult(true);
      return;
    }
    const prompt = new YesNoPrompt(this.ui, this.audioEngine);
    this.dialogue.open(text);
    this.dialogueVisible = true;
    this.tmhmYesNoPrompt = { prompt, onResult };
  }

  beginSaveFlow(options: {
    saveExists: boolean;
    saveCallback: () => boolean | Promise<boolean>;
  }): void {
    if (this.saveFlow) {
      return;
    }
    logger.info(
      `[menu] Begin save flow (saveExists=${String(options.saveExists)})`
    );
    const promptLabel = options.saveExists
      ? "_AlreadyASaveFileText"
      : "_WouldYouLikeToSaveTheGameText";
    const promptText = this.formatSaveText(promptLabel);
    this.dialogue.open(promptText);
    this.dialogueVisible = true;
    this.saveFlow = {
      stage: "prompt",
      prompt: new YesNoPrompt(this.ui, this.audioEngine),
      saveCallback: options.saveCallback,
      saved: false,
    };
  }

  private handleSaveFlowInput(event: KeyEvent): boolean {
    const flow = this.saveFlow;
    if (!flow) {
      return false;
    }
    if (flow.stage === "prompt") {
      this.dialogue.handle_input(event);
      flow.prompt?.handle_input(event);
      if (!flow.prompt || !flow.prompt.finished) {
        return true;
      }
      const confirmed = flow.prompt.result();
      flow.prompt = null;
      logger.info(`[menu] Save prompt result=${String(confirmed)}`);
      if (!confirmed) {
        this.dialogueVisible = false;
        this.dialogue.clear();
        this.saveFlow = null;
        return true;
      }
      const savingText = this.formatSaveText("_SavingDontTurnOffThePowerText");
      this.dialogue.open(savingText);
      this.dialogue.complete();
      this.dialogue.update();
      this.dialogue.draw();
      if (gameEngine.display.get_init()) {
        gameEngine.display.flip();
      }
      const pendingFlow = {
        stage: "saving" as const,
        prompt: null,
        saveCallback: flow.saveCallback,
        saved: false,
      };
      this.saveFlow = pendingFlow;
      void this.completeSaveFlow(pendingFlow);
      return true;
    }

    if (flow.stage === "saving") {
      return true;
    }

    if (flow.stage === "saved") {
      if (!isKeyDownEvent(event)) {
        return true;
      }
      if (!isConfirmEvent(event) && !isCancelEvent(event)) {
        return true;
      }
      this.dialogue.handle_input(event);
      if (!this.dialogue.is_complete() || this.dialogue.has_more_pages()) {
        return true;
      }
      this.dialogueVisible = false;
      this.dialogue.clear();
      if (flow.saved) {
        this.requestMenuClose();
      }
      logger.info(`[menu] Save flow end (saved=${String(flow.saved)})`);
      this.saveFlow = null;
      return true;
    }
    return true;
  }

  private dismissDialogue(): void {
    if (!this.dialogueVisible) {
      return;
    }
    if (!this.dialogue.is_complete()) {
      this.dialogue.complete();
      return;
    }
    if (this.dialogue.has_more_pages()) {
      this.dialogue.advance_page();
      return;
    }
    this.dialogueVisible = false;
    this.onDialogueClosed();
    this.runDialogueCallbacks();
  }

  private queueDialogueCallback(callback: () => void): void {
    this.dialogueAfterClose.push(callback);
  }

  private runDialogueCallbacks(): void {
    if (this.dialogueVisible) {
      return;
    }
    const callbacks = [...this.dialogueAfterClose];
    this.dialogueAfterClose = [];
    callbacks.forEach((callback) => callback());
  }

  private onDialogueClosed(): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    if (ctx.stage === TMHMStage.INTRO_BOOT) {
      ctx.stage = TMHMStage.INTRO_MOVE;
      const moveText = this.formatMoveName(ctx.move);
      this.dialogue.open(`It contained ${moveText}.`);
      this.dialogueVisible = true;
      return;
    }
    if (ctx.stage === TMHMStage.INTRO_MOVE) {
      this.promptTmhmUse();
      return;
    }
    if (ctx.stage === TMHMStage.MESSAGE) {
      const nextStage = ctx.nextStage;
      ctx.nextStage = null;
      if (nextStage === TMHMStage.CHOOSE_POKEMON) {
        this.resumeTmhmSelection();
        return;
      }
      if (nextStage === TMHMStage.FORGET_MENU) {
        this.openForgetMenu(ctx.target);
        return;
      }
      if (nextStage === TMHMStage.COMPLETE) {
        const success = Boolean(ctx.pendingCompletion);
        this.finishTmhmUse(success);
      }
      if (nextStage === TMHMStage.FORGET_PROMPT) {
        this.openForgetMenu(ctx.target);
      }
    }
  }

  private handleBagSelection(selection: [string, string]): void {
    const [action, itemName] = selection;
    if (action === "sel" || action === "register") {
      this.registerKeyItem(itemName);
      return;
    }
    if (action !== "use") {
      return;
    }
    const canonical = MenuState.canonicalItemName(itemName);
    const item = this.itemSystem.getItemDefinition(canonical);
    if (REPEL_DURATIONS[canonical]) {
      if (this.attemptUseRepel(canonical)) {
        return;
      }
    }
    if (EVOLUTION_STONES.has(canonical)) {
      this.beginEvolutionItemUse(canonical);
      return;
    }
    try {
      const [move, isHm] = tmhmSystem.resolveTmhmMove(canonical);
      this.beginTmhmUse(canonical, move, isHm);
      return;
    } catch (error) {
      if (item && this.fieldItemHandler.canHandle(item)) {
        this.ensurePokemonMenu();
        this.fieldItemHandler.begin(item);
        return;
      }
      this.handleKeyItemUse(itemName);
    }
  }

  private promptItemSelectionFromBag(_pokemon: Pokemon): string | Promise<string | null> | null {
    return this.promptItemSelectionFromBagAsync(_pokemon);
  }

  private async promptItemSelectionFromBagAsync(_pokemon: Pokemon): Promise<string | null> {
    if (!this.ui.screen || !gameEngine.display.get_init()) {
      throw new Error("Pokemon menu GIVE requires an initialised display.");
    }
    const bagMenu = new BagMenu(this.ui, this.gameState, this.audioEngine, this.dataLoader ?? undefined, [
      "GIVE",
      "QUIT",
    ]);
    this.waitingForBagItemSelection = true;
    try {
      while (true) {
        for (const event of gameEngine.event.get(this.ui?.eventQueue)) {
          if (event.type === gameEngine.QUIT) {
            gameEngine.quit();
            throw new Error("Quit requested.");
          }
          const result = bagMenu.handleInput(event);
          if (!result) {
            continue;
          }
          const [action, itemName] = result;
          const normalized = action.toLowerCase();
          if (normalized === "give") {
            return itemName;
          }
          if (normalized === "cancel" || normalized === "quit") {
            return null;
          }
        }
        bagMenu.draw();
        if (gameEngine.display.get_init()) {
          gameEngine.display.flip();
        }
        await nextFrame();
      }
    } finally {
      this.waitingForBagItemSelection = false;
    }
  }

  private handleKeyItemUse(itemName: string): void {
    if (!this.overworld?.use_key_item) {
      return;
    }
    const result = this.overworld.use_key_item(itemName);
    if (result instanceof Promise) {
      this.requestMenuClose();
      result.catch((error) => {
        logger.error("[menu] Key item failed", error);
      });
      return;
    }
    if (result) {
      this.requestMenuClose();
      return;
    }
    this.showCantUseKeyItemText();
  }

  private showCantUseKeyItemText(): void {
    if (!this.overworld) {
      return;
    }
    const showTextAsync = this.overworld._show_field_move_text_async;
    if (typeof showTextAsync === "function") {
      this.requestMenuClose();
      showTextAsync.call(this.overworld, "CantUseItemText").catch((error) => {
        logger.error("[menu] Key item failure text failed", error);
      });
      return;
    }
    this.overworld._show_field_move_text?.("CantUseItemText");
  }

  private registerKeyItem(itemName: string): void {
    // ASM: pokecrystal_disassembly/engine/items/pack.asm::RegisterItem
    const canonical = MenuState.canonicalItemName(itemName);
    const pocket = this.itemSystem.getItemPocket(canonical);
    if (pocket !== ItemPocket.KEY_ITEM) {
      this.showRegisterText("_CantRegisterText");
      return;
    }
    const bagMenu = this.bagMenu ?? this.ensureBagMenu();
    const pocketIndex = bagMenu.getPocketIndex();
    if (pocketIndex < 0 || pocketIndex > 3) {
      throw new Error(`Invalid pocket index for registered item: ${pocketIndex}`);
    }
    const registeredNumber = bagMenu.getListIndex() + 1;
    if (registeredNumber <= 0 || registeredNumber > 0x3f) {
      throw new Error(`Registered item index out of range: ${registeredNumber}`);
    }
    this.gameState.wram.wWhichRegisteredItem = ((pocketIndex & 0x03) << 6) | (registeredNumber & 0x3f);
    this.gameState.wram.wRegisteredItem = canonical;
    this.showRegisterText("_RegisteredItemText", {
      STRING_BUFFER_2: this.itemSystem.getDisplayName(canonical).toUpperCase(),
    });
  }

  private showRegisterText(label: string, buffers?: Record<string, string>): void {
    const raw = asmTextLoader.get(label);
    if (!raw) {
      throw new Error(`Missing ASM text for label '${label}'.`);
    }
    const formatter = new TextFormatter(this.gameState);
    if (buffers) {
      formatter.stringBuffers = { ...buffers };
    }
    this.dialogue.open(formatter.formatText(raw));
    this.dialogueVisible = true;
  }

  private attemptUseRepel(canonicalItem: string): boolean {
    if (!this.overworld) {
      return false;
    }
    const duration = REPEL_DURATIONS[canonicalItem];
    if (!duration) {
      return false;
    }
    const wram = this.gameState.wram;
    if (Number(wram.repel_steps ?? 0) > 0) {
      this.overworld._show_field_move_text?.("RepelUsedEarlierIsStillInEffectText");
      return true;
    }
    if (!this.itemSystem.removeItem(canonicalItem)) {
      return false;
    }
    wram.repel_steps = duration;
    this.overworld._show_field_move_text?.("RepelUseText");
    this.requestMenuClose();
    return true;
  }

  private static canonicalItemName(itemName: string): string {
    return itemName.replace(/ /g, "_").toUpperCase();
  }

  private promptTmhmUse(): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    const moveText = this.formatMoveName(ctx.move);
    const prompt = `Teach ${moveText}?`;
    this.startTmhmYesNoPrompt(prompt, (confirmed) => {
      if (confirmed) {
        this.startTmhmSelection();
        return;
      }
      this.showTmhmMessage("The TM wasn't used.", TMHMStage.COMPLETE, false);
    });
  }

  private beginTmhmUse(itemName: string, move: MoveName, isHm: boolean): void {
    if (this.tmhmContext) {
      return;
    }
    this.tmhmContext = {
      itemName,
      move,
      isHm,
      stage: TMHMStage.INTRO_BOOT,
      nextStage: null,
      target: null,
      pendingCompletion: null,
      forgetOptions: [],
    };
    const intro = isHm ? "Booted up an HM." : "Booted up a TM.";
    this.dialogue.open(intro);
    this.dialogueVisible = true;
  }

  private startTmhmSelection(): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    ctx.stage = TMHMStage.CHOOSE_POKEMON;
    ctx.nextStage = null;
    ctx.pendingCompletion = null;
    this.ensurePokemonMenu().reset();
    this.currentMenu = "pokemon_menu";
    this.ensurePokemonMenu().setAction(PartyMenuAction.TEACH_TMHM);
    this.ensurePokemonMenu().setTmhmMove(ctx.move);
    this.ensurePokemonMenu().requestSelection({
      handler: (pokemon) => {
        this.handleTmhmSelection(pokemon);
        return false;
      },
      cancelHandler: () => this.cancelTmhmSelection(),
    });
  }

  private resumeTmhmSelection(): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    ctx.stage = TMHMStage.CHOOSE_POKEMON;
    this.currentMenu = "pokemon_menu";
  }

  private handleTmhmSelection(pokemon: Pokemon): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    const moveText = this.formatMoveName(ctx.move);
    if (MenuState.isEgg(pokemon)) {
      this.playTmhmSound("SFX_WRONG");
      this.showTmhmMessage("Eggs can't learn any moves.", TMHMStage.CHOOSE_POKEMON);
      return;
    }
    if (!tmhmSystem.pokemonCanLearnTmhm(pokemon, ctx.move)) {
      this.playTmhmSound("SFX_WRONG");
      this.showTmhmMessage(`${pokemon.nickname} can't learn ${moveText}.`, TMHMStage.CHOOSE_POKEMON);
      return;
    }
    if (tmhmSystem.pokemonKnowsMove(pokemon, ctx.move)) {
      this.playTmhmSound("SFX_WRONG");
      this.showTmhmMessage(`${pokemon.nickname} already knows ${moveText}.`, TMHMStage.CHOOSE_POKEMON);
      return;
    }
    const moves = pokemon.moves.filter((entry) => entry) as Array<{ name: MoveName }>;
    if (moves.length < 4) {
      this.teachTmhmMove(pokemon);
      return;
    }
    ctx.target = pokemon;
    this.confirmForgetPrompt(pokemon);
  }

  private cancelTmhmSelection(): void {
    this.showTmhmMessage("The TM wasn't used.", TMHMStage.COMPLETE, false);
  }

  private confirmForgetPrompt(pokemon: Pokemon): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    const moveText = this.formatMoveName(ctx.move);
    const prompt = `Delete a move to make room for ${moveText}?`;
    this.startTmhmYesNoPrompt(prompt, (confirmed) => {
      const activeCtx = this.tmhmContext;
      if (!activeCtx) {
        return;
      }
      if (confirmed) {
        activeCtx.stage = TMHMStage.FORGET_PROMPT;
        this.openForgetMenu(pokemon);
        return;
      }
      this.promptStopLearning(pokemon, moveText);
    });
  }

  private promptStopLearning(pokemon: Pokemon, moveText: string): void {
    this.startTmhmYesNoPrompt(`Stop learning ${moveText}?`, (stopLearning) => {
      if (stopLearning) {
        this.showTmhmMessage(`${pokemon.nickname} did not learn ${moveText}.`, TMHMStage.COMPLETE, false);
        return;
      }
      this.openForgetMenu(pokemon);
    });
  }

  private openForgetMenu(pokemon: Pokemon | null): void {
    const ctx = this.tmhmContext;
    if (!ctx || !pokemon) {
      return;
    }
    const options = pokemon.moves
      .filter((move) => move)
      .map((move) => (move as { name: MoveName }).name.replace(/_/g, " "));
    options.push("CANCEL");
    ctx.forgetOptions = options;
    const tile = this.ui.tileSize;
    this.activeOptionMenu = new Menu(this.ui, options, tile * 5, tile * 2, 15, 10, 1, undefined, null, this.audioEngine);
    ctx.stage = TMHMStage.FORGET_MENU;
  }

  private handleForgetMenuChoice(choice: string): void {
    const ctx = this.tmhmContext;
    const pokemon = ctx?.target ?? null;
    this.activeOptionMenu = null;
    if (!ctx || !pokemon) {
      return;
    }
    const moveText = this.formatMoveName(ctx.move);
    if (choice === "CANCEL") {
      this.showTmhmMessage(`${pokemon.nickname} did not learn ${moveText}.`, TMHMStage.CHOOSE_POKEMON);
      return;
    }
    const index = ctx.forgetOptions.indexOf(choice);
    if (index === -1) {
      this.showTmhmMessage("Couldn't determine which move to forget.", TMHMStage.CHOOSE_POKEMON);
      return;
    }
    this.teachTmhmMove(pokemon, index);
  }

  private teachTmhmMove(pokemon: Pokemon, replaceIndex?: number): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    const moveData = this.resolveMoveData(ctx.move);
    try {
      tmhmSystem.learnMove(pokemon, ctx.move, moveData, replaceIndex);
    } catch (error) {
      this.showTmhmMessage(String(error), TMHMStage.CHOOSE_POKEMON);
      return;
    }
    ctx.target = pokemon;
    this.showTmhmMessage(
      `${pokemon.nickname} learned ${this.formatMoveName(ctx.move)}!`,
      TMHMStage.COMPLETE,
      true,
    );
  }

  private resolveMoveData(move: MoveName): Move | undefined {
    if (!this.dataLoader) {
      return undefined;
    }
    if (!this.dataLoader.move_data && this.dataLoader.load_move_data) {
      this.dataLoader.load_move_data();
    }
    const data = this.dataLoader.move_data ?? {};
    return data[move] || data[String(move)] || undefined;
  }

  private finishTmhmUse(success: boolean): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    if (success) {
      tmhmSystem.consumeTmhmItem(this.itemSystem, ctx.itemName, { isHm: ctx.isHm });
    }
    this.tmhmContext = null;
    this.ensurePokemonMenu().clearSelectionRequest();
    this.ensurePokemonMenu().setTmhmMove(null);
    this.currentMenu = "bag_menu";
  }

  private beginEvolutionItemUse(itemName: string): void {
    this.evoHandler.begin(itemName);
  }

  private playTmhmSound(token: string): void {
    try {
      this.audioEngine.playSound(token);
    } catch {
      return;
    }
  }

  public playSound(token: string): void {
    try {
      this.audioEngine.playSound(token);
    } catch {
      return;
    }
  }

  public play_sound(token: string): void {
    this.playSound(token);
  }

  private formatSaveText(label: string): string {
    const raw = label ? asmTextLoader.get(label) : "";
    if (!raw) {
      throw new Error(`Missing ASM text for label '${label}'.`);
    }
    const playerName = String(this.gameState.sram.player_name ?? "").trim() || "PLAYER";
    const rivalName = String(this.gameState.sram.rival_name ?? "").trim() || "RIVAL";
    return raw
      .replace(/<PLAYER>/g, playerName)
      .replace(/<PLAY_G>/g, playerName)
      .replace(/<RIVAL>/g, rivalName);
  }

  private renderSavingText(text: string): void {
    if (!text) {
      return;
    }
    logger.info("[menu] renderSavingText start");
    this.dialogue.open(text);
    this.dialogueVisible = true;
    if (!gameEngine.display.get_init()) {
      logger.info("[menu] renderSavingText bypassed (display not initialized)");
      this.dialogue.complete();
      this.dialogueVisible = false;
      this.dialogue.clear();
      return;
    }
    this.dialogue.complete();
    this.dialogue.update();
    this.dialogue.draw();
    gameEngine.display.flip();
    logger.info("[menu] renderSavingText complete");
  }

  private blockingDialogue(text: string): void {
    if (!text) {
      return;
    }
    logger.info("[menu] blockingDialogue start");
    this.dialogue.open(text);
    this.dialogueVisible = true;
    if (!gameEngine.display.get_init()) {
      logger.info("[menu] blockingDialogue bypassed (display not initialized)");
      this.dialogue.complete();
      this.dialogueVisible = false;
      this.dialogue.clear();
      return;
    }
    this.dialogue.complete();
    this.dialogue.update();
    gameEngine.display.flip();
    this.dialogueVisible = false;
    this.dialogue.clear();
  }

  runSaveFlow(options: {
    saveExists: boolean;
    saveCallback: () => boolean | Promise<boolean>;
  }): boolean {
    this.beginSaveFlow(options);
    return false;
  }

  private async completeSaveFlow(flow: NonNullable<MenuState["saveFlow"]>): Promise<void> {
    await nextFrame();
    if (this.saveFlow !== flow || flow.stage !== "saving") {
      return;
    }

    try {
      logger.info("[menu] Save callback begin");
      const saved = Boolean(await flow.saveCallback());
      logger.info(`[menu] Save callback complete (saved=${String(saved)})`);
      const savedText = this.formatSaveText("_SavedTheGameText");
      this.dialogue.open(savedText);
      this.dialogueVisible = true;
      this.saveFlow = {
        stage: "saved",
        prompt: null,
        saveCallback: flow.saveCallback,
        saved,
      };
    } catch (error) {
      logger.error(`[menu] Save callback error: ${String(error)}`);
      const errorText = this.formatSaveText("_SaveFileCorruptedText");
      this.dialogue.open(errorText);
      this.dialogueVisible = true;
      this.saveFlow = null;
      if (!(error instanceof SaveGameError)) {
        logger.error("[menu] Unexpected async save error treated as save failure");
      }
    }
  }

  private showTmhmMessage(text: string, nextStage: TMHMStage, completion: boolean | null = null): void {
    const ctx = this.tmhmContext;
    if (!ctx) {
      return;
    }
    ctx.stage = TMHMStage.MESSAGE;
    ctx.nextStage = nextStage;
    ctx.pendingCompletion = completion ?? false;
    this.dialogue.open(text);
    this.dialogueVisible = true;
  }

  private formatMoveName(move: MoveName): string {
    return String(move).replace(/_/g, " ");
  }

  private static isEgg(pokemon: Pokemon): boolean {
    const species = String(pokemon.species?.id ?? "").toUpperCase();
    const nickname = String(pokemon.nickname ?? "").toUpperCase();
    return species === "EGG" || nickname === "EGG";
  }
}
