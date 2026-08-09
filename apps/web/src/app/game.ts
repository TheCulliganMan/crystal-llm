import "@/lib/pokecrystal-core/register-browser-adapters";
import { GameState, createInitialGameState } from "@pokecrystal/core/core/state";
import { VRAMManager } from "@pokecrystal/core/core/memory/vram";
import { createCircuitBreaker } from "@pokecrystal/core/utils";
import {
  hasSaveGame,
  loadGame,
  saveGame,
  saveGameWithHistory,
  deleteSaveGame,
  SaveFileNotFoundError,
} from "@pokecrystal/core/core/save";
import logger from "@pokecrystal/core/core/logger";
import { Event, EventManager, StartBattleEvent } from "@pokecrystal/core/engine/events/events";
import type { StartBattleEventPayload } from "@pokecrystal/core/engine/events/events";
import { PlayerGender, TimeOfDay } from "@pokecrystal/core/core/enums";
import { OverworldEngine as Overworld } from "@pokecrystal/core/engine/world/overworld/overworld";
import { DataLoader, preloadCoreDataAssets } from "@pokecrystal/core/core/data-loader";
import { AudioEngine, type AudioPlaybackSnapshot } from "@pokecrystal/core/engine/systems/audio";
import { DAY_HOUR, MORN_HOUR, NITE_HOUR, TimeSystem } from "@pokecrystal/core/engine/systems/time";
import { OverworldTileset } from "@pokecrystal/core/engine/world/overworld/overworld-tileset";
import { primeStoryEventRuntimeAssets } from "@pokecrystal/core/engine/world/story-events/common";
import { PokemonSchema, toPokemon, type Pokemon } from "@pokecrystal/core/core/models";
import type { OverworldTilesetLike } from "@pokecrystal/core/engine/world/overworld/tileset-types";
import type { RemoteOverworldPlayer } from "@pokecrystal/core/types/overworld";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import {
  begin_battle,
  create_battle_ui as createBattleUI,
  end_battle,
  set_game_state,
  set_audio_engine,
} from "@pokecrystal/core/ui/overlays/battle-ui";
import { BattleUILayoutFactory } from "@pokecrystal/core/ui/overlays/_battle-layout";
import { MenuState } from "@pokecrystal/core/ui/menus/menu-state";
import { MainMenu } from "@pokecrystal/core/ui/menus/main-menu";
import { Battle } from "@pokecrystal/core/engine/battle/battle/battle-logic";
import { TrainerBattle } from "@pokecrystal/core/engine/battle/battle/trainer-battle";
import { MultiplayerBattle } from "@pokecrystal/core/multiplayer/multiplayer-battle";
import type { BattleSyncTransport } from "@pokecrystal/core/multiplayer/battle-synchronizer";
import { BaseUI } from "@pokecrystal/core/ui/base-ui";
import type { BaseFontRenderer } from "@pokecrystal/core/ui/base-ui";
import { Surface } from "@pokecrystal/core/ui/surface";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { TextRenderer } from "@pokecrystal/core/ui/text/text-renderer";
import { TextUI } from "@pokecrystal/core/ui/text-ui";
import type { FontRenderer as TextboxFontRenderer } from "@pokecrystal/core/ui/textbox";
import { isStartEvent } from "@pokecrystal/core/input/buttons";
import {
  isKeyDownEvent,
  isKeyUpEvent,
  isSelectEvent,
  mapKeyToButton,
  mapKeyToDirection,
} from "@pokecrystal/core/input/controls";
import { applySpawn, getMapMetadataByGroup, Spawn } from "@pokecrystal/core/engine/world/maps";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { joypad_bits_for_event, resetJoypadFrame } from "@pokecrystal/core/input/joypad";
import fs from "fs";
import { IntroSequence } from "@pokecrystal/core/ui/screens/intro/intro-sequence";
import { TitleScreen, TitleScreenOption } from "@pokecrystal/core/ui/screens/title-screen";
import { ContinueScreen } from "@pokecrystal/core/ui/screens/continue-screen";
import { DeleteSaveScreen } from "@pokecrystal/core/ui/screens/delete-save-screen";
import { ClockResetScreen } from "@pokecrystal/core/ui/screens/clock-reset-screen";
import { OakIntroSequence } from "@pokecrystal/core/ui/screens/intro/oak-intro-sequence";
import { GenderSelectionScreen } from "@pokecrystal/core/ui/screens/intro/gender-selection";
import { NameEntryScreen } from "@pokecrystal/core/ui/screens/name-entry-screen";
import type { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import type { BattleUI } from "@pokecrystal/core/ui/overlays/battle-ui-core";
import type { BattleUIState } from "@pokecrystal/core/ui/overlays/battle-ui-state";
import type { MenuUI, FontRenderer as MenuFontRenderer } from "@pokecrystal/core/ui/menus/types";
import { GameBenchmark, type GameBenchmarkOptions } from "@/app/game-benchmark";
import { assertAsmUiInvariants } from "@/app/asm-rendering-invariants";
import { determineBattleMusic } from "@pokecrystal/core/engine/battle/battle/music";
import { WhiteoutManager } from "@pokecrystal/core/engine/world/whiteout";
import {
  AUTOSAVE_SLOT,
  MANUAL_SAVE_HISTORY_SLOTS,
  MANUAL_SAVE_SLOT,
  MANUAL_SAVE_SLOTS,
} from "@pokecrystal/core/core/save-slots";
import { setUnownPuzzleAssetLoader } from "@pokecrystal/assets/content/data/unown-puzzles";
import { getUnownOverlayLockDepth } from "@pokecrystal/core/engine/world/special-events/unown-overlay-lock";
import { readJsonAsset } from "@pokecrystal/core/core/asset-reader";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { renderTextSnapshot } from "@pokecrystal/core/ui/text-overlays";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";

type GameStateName =
  | "intro"
  | "title"
  | "main_menu"
  | "continue"
  | "delete_save"
  | "clock_reset"
  | "gender"
  | "oak_intro"
  | "name_entry"
  | "overworld"
  | "battle"
  | "menu";

type GameOptions = {
  initialState?: GameStateName;
  muted?: boolean;
  masterVolume?: number;
  loadSlot?: string;
  strictLoadSlot?: boolean;
  autosaveSlot?: string;
  benchmarking?: GameBenchmarkOptions;
  initialSpawnId?: Spawn;
  playIntro?: boolean;
  newGame?: boolean;
  suppressBootAnimations?: boolean;
  bootSaveSlot?: string;
  onLoadProgress?: (progress: GameLoadProgress) => void;
  preloadMode?: GamePreloadMode;
};

// ASM/hardware cadence: one frame is one VBlank period (70,224 cycles).
const FRAME_DURATION_MS = GB_FRAME_DURATION_MS;
const MAX_ACCUMULATED_MS = FRAME_DURATION_MS * 5;
const MAX_TICKS_PER_FRAME = Math.max(1, Math.floor(MAX_ACCUMULATED_MS / FRAME_DURATION_MS));
const TIME_OF_DAY_HOURS: Record<TimeOfDay, number> = {
  [TimeOfDay.MORN]: MORN_HOUR,
  [TimeOfDay.DAY]: DAY_HOUR,
  [TimeOfDay.NIGHT]: NITE_HOUR,
};
const AUTOSAVE_STEP_THRESHOLD = 250;
const DEFAULT_BOOT_SLOT = MANUAL_SAVE_SLOT;
const DEFAULT_PLAYER_NAME = "CHRIS";
const SPRITE_PRELOAD_IDS = ["chris", "kris"] as const;
const SPRITE_EXT = ".png";
const BOOT_SPRITE_PATHS = SPRITE_PRELOAD_IDS.map((id) =>
  getAssetPath("gfx", "sprites", `${id}${SPRITE_EXT}`)
);
const BOOT_PC_PATHS = [
  getAssetPath("gfx", "pc", `pc${SPRITE_EXT}`),
  getAssetPath("gfx", "pc", `pc_mail${SPRITE_EXT}`),
];
const BOOT_BATTLE_PATHS = [
  // ASM parity: engine/battle/trainer_huds.asm::LoadBallIconGFX synchronously
  // stages the party / status / fainted / empty ball icons before drawing the HUD.
  getAssetPath("gfx", "battle", "balls.png"),
];
const BOOT_OVERWORLD_PATHS = [
  getAssetPath("gfx", "overworld", `grass_rustle${SPRITE_EXT}`),
  getAssetPath("gfx", "overworld", `heal_machine${SPRITE_EXT}`),
  // ASM parity: overworld emotes are rendered synchronously, so these sprites
  // must be in the boot preload set rather than loading on-demand.
  getAssetPath("gfx", "emotes", `bolt${SPRITE_EXT}`),
  getAssetPath("gfx", "emotes", `fish${SPRITE_EXT}`),
  getAssetPath("gfx", "emotes", `happy${SPRITE_EXT}`),
  getAssetPath("gfx", "emotes", `heart${SPRITE_EXT}`),
  getAssetPath("gfx", "emotes", `question${SPRITE_EXT}`),
  getAssetPath("gfx", "emotes", `sad${SPRITE_EXT}`),
  getAssetPath("gfx", "emotes", `shock${SPRITE_EXT}`),
  getAssetPath("gfx", "emotes", `sleep${SPRITE_EXT}`),
];
const BOOT_NAMING_SCREEN_PATHS = [
  getAssetPath("gfx", "font", `font${SPRITE_EXT}`),
  getAssetPath("gfx", "naming_screen", `border${SPRITE_EXT}`),
  getAssetPath("gfx", "naming_screen", `cursor${SPRITE_EXT}`),
  getAssetPath("gfx", "naming_screen", `underline${SPRITE_EXT}`),
  getAssetPath("gfx", "naming_screen", `middle_line${SPRITE_EXT}`),
];
const BOOT_INTRO_PATHS = [
  getAssetPath("gfx", "intro", `unowns${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `background${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `suicune_run${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `pulse${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `crystal_unowns${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `pichu_wooper${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `suicune_close${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `suicune_jump${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `suicune_back${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `unown_back${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `grass1${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `grass2${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `grass3${SPRITE_EXT}`),
  getAssetPath("gfx", "intro", `grass4${SPRITE_EXT}`),
];
const BOOT_TRAINER_CARD_PATHS = [
  getAssetPath("gfx", "trainer_card", `chris_card${SPRITE_EXT}`),
  getAssetPath("gfx", "trainer_card", `kris_card${SPRITE_EXT}`),
];
const FALLBACK_FRAME_IDS = [1];
const PRELOAD_DATA_SUFFIXES = [
  ".pal",
  ".gbcpal",
  ".bin",
  ".2bpp",
  ".1bpp",
  ".2bpp.lz",
  ".1bpp.lz",
  ".tilemap",
  ".tilemap.rle",
  ".attrmap",
  ".rle",
];
const PRELOAD_DATA_PREFIXES = ["/assets/data"];
const PRELOAD_BATTLE_ANIM_SUFFIXES = [".2bpp", ".pal"];
const PRELOAD_BATTLE_ANIM_PREFIXES = ["/assets/gfx/battle_anims"];
const PRELOAD_SPRITE_SUFFIXES = [".png"];
const PRELOAD_SPRITE_PREFIXES = ["/assets/gfx/sprites"];
const BLOCKING_PREFETCH_CONCURRENCY = 24;
const DEFERRED_PREFETCH_CONCURRENCY = 8;
const UNOWN_PUZZLE_BINARY_ASSETS = [
  getAssetPath("gfx", "unown_puzzle", "cursor.2bpp"),
  getAssetPath("gfx", "unown_puzzle", "start_cancel.2bpp"),
  getAssetPath("gfx", "unown_puzzle", "tile_borders.2bpp"),
  getAssetPath("gfx", "unown_puzzle", "kabuto.2bpp"),
  getAssetPath("gfx", "unown_puzzle", "omanyte.2bpp"),
  getAssetPath("gfx", "unown_puzzle", "aerodactyl.2bpp"),
  getAssetPath("gfx", "unown_puzzle", "hooh.2bpp"),
] as const;
const UNOWN_PUZZLE_JSON_ASSETS = [
  getAssetPath("data", "unown_puzzles", "coordinates.json"),
  getAssetPath("data", "unown_puzzles", "layouts.json"),
] as const;

type CoreDataPrefetchMode = "none" | "deferred" | "blocking";

const resolveUnownPuzzleAssetPath = (assetPathName: string): string => {
  const normalized = assetPathName.replace(/\\/g, "/");
  if (!normalized) {
    return normalized;
  }
  if (/^\//.test(normalized)) {
    return normalized;
  }
  if (normalized.startsWith("assets/")) {
    return getAssetPath(...normalized.replace(/^assets\//, "").split("/"));
  }
  return getAssetPath(normalized);
};

let unownPuzzleAssetLoaderRegistered = false;
let coreAssetsPreloadState: "idle" | "loading" | "ready" = "idle";
let coreAssetsPreloadPromise: Promise<void> | null = null;
const UNOWN_PUZZLE_ASSET_TOTAL = UNOWN_PUZZLE_BINARY_ASSETS.length + UNOWN_PUZZLE_JSON_ASSETS.length;

const preloadUnownPuzzleAssets = async (
  onProgress?: (completed: number, total: number, label?: string) => void,
  progressState?: { completed: number; total: number }
): Promise<void> => {
  const reportProgress = (path: string): void => {
    if (!onProgress || !progressState) {
      return;
    }
    progressState.completed += 1;
    onProgress(progressState.completed, progressState.total, path);
  };
  await Promise.all([
    ...UNOWN_PUZZLE_BINARY_ASSETS.map((path) =>
      fs.promises.readFile(path).catch((error: unknown) => {
        logger.warn("[game] Failed to prewarm Unown puzzle binary asset", { path, error });
      }).finally(() => {
        reportProgress(path);
      })
    ),
    ...UNOWN_PUZZLE_JSON_ASSETS.map((path) =>
      readJsonAsset(path).catch((error: unknown) => {
        logger.warn("[game] Failed to prewarm Unown puzzle data asset", { path, error });
      }).finally(() => {
        reportProgress(path);
      })
    ),
  ]);
};

const resolveCoreDataPrefetchMode = (): CoreDataPrefetchMode => {
  const normalized = String(process.env.NEXT_PUBLIC_CORE_DATA_PREFETCH_MODE ?? "deferred")
    .trim()
    .toLowerCase();
  if (normalized === "none" || normalized === "deferred" || normalized === "blocking") {
    return normalized;
  }
  return "deferred";
};

const registerUnownPuzzleAssetLoader = (): void => {
  if (unownPuzzleAssetLoaderRegistered) {
    return;
  }
  setUnownPuzzleAssetLoader((assetPathName: string) => {
    const path = resolveUnownPuzzleAssetPath(assetPathName);
    return fs.readFileSync(path);
  });
  unownPuzzleAssetLoaderRegistered = true;
};

const resolveBootSaveSlot = (slot?: string): string => {
  const trimmed = slot?.trim();
  if (trimmed) {
    return trimmed;
  }
  if (typeof process !== "undefined") {
    const envSlot = process.env.NEXT_PUBLIC_LOAD_SLOT?.trim();
    if (envSlot) {
      return envSlot;
    }
  }
  return DEFAULT_BOOT_SLOT;
};

const resolveBootSaveCandidates = (slot: string): string[] => {
  if (MANUAL_SAVE_SLOTS.includes(slot)) {
    return [...MANUAL_SAVE_SLOTS];
  }
  return [slot];
};

export type GameLoadPhase = "core-assets" | "core-data" | "ready";

export type GamePreloadMode = "auto" | "none";

export type GameLoadProgress = {
  phase: GameLoadPhase;
  completed: number;
  total: number;
  ratio: number;
  label?: string;
};

export type GameDebugScene =
  | "intro"
  | "title"
  | "main_menu"
  | "continue"
  | "delete_save"
  | "clock_reset"
  | "gender"
  | "oak_intro"
  | "name_entry"
  | "new_game"
  | "overworld";

export type GameDebugStatus = {
  mode: GameStateName;
  mapName: string;
  mapGroup: number;
  mapNumber: number;
  coords: {
    x: number;
    y: number;
  };
  prompt_pending: boolean;
  text_advance_pending: boolean;
  prompt_reason: string | null;
  in_dialog: boolean;
  in_menu: boolean;
  in_battle: boolean;
  movement_locked: boolean;
  script_busy: boolean;
  can_move: boolean;
  current_spawn: number | null;
  oak_intro?: {
    mode: "intro" | "final";
    sceneIndex: number;
    sceneState: string;
    scenePhase: string;
    currentSprite: string | null;
    waitingForInput: boolean;
    timeSetComplete: boolean;
  } | null;
  intro?: {
    sceneIndex: number;
    sceneName: string;
    sceneFrameCounter: number;
    spriteCount: number;
    scrollX: number;
    scrollY: number;
    finished: boolean;
  } | null;
  name_entry?: {
    finished: boolean;
    name: string;
  } | null;
};

type BGMapSyncRuntime = {
  bg_map_sync?: { is_busy: boolean; remaining_frames?: number };
  write_bg_map_with_wait?: (
    name: string,
    width: number,
    height: number,
    tiles: number[],
    attrs: number[],
    options?: { origin_x?: number; origin_y?: number }
  ) => void;
};

export type MultiplayerBattleCompleteResult = {
  result: number;
};

export class Game {
  public static reset_preload_state_for_tests(): void {
    coreAssetsPreloadState = "idle";
    coreAssetsPreloadPromise = null;
  }

  private gameState: GameState;
  private eventManager: EventManager;
  private dataLoader: DataLoader;
  private audioEngine: AudioEngine;
  private overworld: Overworld;
  private battleUi: BattleUIState;
  private menuState: MenuState;
  private battle: Battle | null = null;
  private multiplayerBattle: MultiplayerBattle | null = null;
  private multiplayerBattleTransport: BattleSyncTransport | null = null;
  private multiplayerIsHost = false;
  private multiplayerBattleCompleteCallback: ((result: MultiplayerBattleCompleteResult) => void) | null = null;
  private tileset: OverworldTilesetLike;
  private initPromise: Promise<void> | null = null;
  private readonly eventQueue = gameEngine.event.createQueue();
  private bootRenderContext: CanvasRenderingContext2D | null = null;
  private vramManager: VRAMManager | null = null;
  private readonly gameLoopBound = (timestamp?: number) => {
    this.gameLoop(timestamp);
  };
  private fatalError: Error | null = null;
  private readonly benchmark: GameBenchmark | null;
  private whiteoutManager: WhiteoutManager;
  private autosaveStepCounter = 0;
  private hasPersistedSaveData = false;
  private readonly autosaveSlot: string;
  private readonly bootSaveSlot: string;
  private readonly bootSequenceMode: "intro" | "title" | "overworld";
  private introSequence: IntroSequence | null = null;
  private titleScreen: TitleScreen | null = null;
  private mainMenu: MainMenu | null = null;
  private continueScreen: ContinueScreen | null = null;
  private deleteSaveScreen: DeleteSaveScreen | null = null;
  private clockResetScreen: ClockResetScreen | null = null;
  private genderSelection: GenderSelectionScreen | null = null;
  private oakIntroSequence: OakIntroSequence | null = null;
  private playerNameEntryScreen: NameEntryScreen | null = null;
  private bootTransitionTask: Promise<void> | null = null;
  private bootTransitionBlackout = false;
  private isOakIntroFinalSequence = false;

  private currentState: GameStateName = "overworld";
  private pendingOverworldMusicAction: "start" | "restart" | null = null;
  private quitRequested = false;
  private lastFrameTimeMs: number | null = null;
  private frameRemainderMs = 0;
  private loopTimerId: ReturnType<typeof setTimeout> | null = null;

  public static async create(ui: BaseUI, options: GameOptions = {}): Promise<Game> {
    registerUnownPuzzleAssetLoader();
    const reportProgress = (phase: GameLoadPhase) => {
      return (completed: number, total: number, label?: string) => {
        const ratio = total > 0 ? completed / total : 1;
        options.onLoadProgress?.({
          phase,
          completed,
          total,
          ratio,
          label,
        });
      };
    };
    const resolvedPreloadMode: GamePreloadMode = options.preloadMode ?? "auto";
    if (resolvedPreloadMode === "auto") {
      await Promise.all([
        Game.preload_core_assets(reportProgress("core-assets")),
        Game.prepare_ui(ui),
        preloadCoreDataAssets("core", { onProgress: reportProgress("core-data") }),
        primeStoryEventRuntimeAssets(),
      ]);
    } else {
      await Game.prepare_ui(ui);
    }
    let loadedState: GameState | null = null;
    if (options.loadSlot) {
      try {
        loadedState = await loadGame(options.loadSlot);
      } catch (error) {
        if (options.strictLoadSlot && !(error instanceof SaveFileNotFoundError)) {
          throw error;
        }
        if (!(error instanceof SaveFileNotFoundError)) {
          console.warn(`[game] Unable to load save slot ${options.loadSlot}`, error);
        }
      }
    }
    let saveSlotExists = loadedState !== null;
    if (!saveSlotExists && options.loadSlot) {
      try {
        saveSlotExists = await hasSaveGame(options.loadSlot);
      } catch (error) {
        logger.warn(`[game] Unable to probe save slot ${options.loadSlot}`, error);
      }
    }
    const game = new Game(ui, options, loadedState ?? undefined);
    if (!saveSlotExists) {
      saveSlotExists = (await game.resolveActiveBootSaveSlot()) !== null;
    }
    game.hasPersistedSaveData = saveSlotExists;
    await game.init();
    await game.initializeBootScreens();
    options.onLoadProgress?.({
      phase: "ready",
      completed: 1,
      total: 1,
      ratio: 1,
    });
    return game;
  }

  private async initializeBootScreens(): Promise<void> {
    if (!this.currentStateHasBootIntro()) {
      return;
    }
    if (this.currentState === "intro") {
      this.ensureIntroSequence();
      this.currentState = "intro";
      this.isOakIntroFinalSequence = false;
      await this.ensureTitleScreen();
      this.titleScreen?.startFromGameStart();
      return;
    }

    await this.ensureTitleScreen();
    if (
      this.currentState !== "clock_reset" &&
      this.currentState !== "gender" &&
      this.currentState !== "oak_intro" &&
      this.currentState !== "name_entry"
    ) {
      this.titleScreen?.startFromGameStart();
    }
    this.isOakIntroFinalSequence = false;
    if (this.currentState === "main_menu") {
      await this.enterMainMenuState();
      return;
    }
    if (this.currentState === "continue") {
      this.ensureContinueScreen();
      return;
    }
    if (this.currentState === "delete_save") {
      this.enterDeleteSaveState();
      return;
    }
    if (this.currentState === "clock_reset") {
      this.enterClockResetState();
      return;
    }
    if (this.currentState === "gender") {
      this.enterGenderState();
      return;
    }
    if (this.currentState === "oak_intro") {
      await this.ensureOakIntroSequence();
      this.currentState = "oak_intro";
      this.isOakIntroFinalSequence = false;
      this.setBootPlayerGender(this.getBootPlayerGender());
      this.oakIntroSequence?.reset();
      return;
    }
    if (this.currentState === "name_entry") {
      this.enterPlayerNameEntryState();
      return;
    }
  }

  private currentStateHasBootIntro(): boolean {
    return (
      this.currentState === "intro" ||
      this.currentState === "title" ||
      this.currentState === "main_menu" ||
      this.currentState === "continue" ||
      this.currentState === "delete_save" ||
      this.currentState === "clock_reset" ||
      this.currentState === "gender" ||
      this.currentState === "oak_intro" ||
      this.currentState === "name_entry"
    );
  }

  private async ensureTitleScreen(): Promise<void> {
    if (!this.titleScreen) {
      this.titleScreen = await TitleScreen.create(this.audioEngine);
    }
  }

  private ensureIntroSequence(): void {
    if (!this.introSequence) {
      this.introSequence = new IntroSequence(this.audioEngine);
    }
  }

  private ensureContinueScreen(): void {
    if (!this.continueScreen) {
      this.continueScreen = new ContinueScreen(
        this.ui as unknown as ScreenUI,
        this.gameState,
        this.audioEngine
      );
    }
  }

  private ensureGenderSelection(): void {
    if (!this.genderSelection) {
      this.genderSelection = new GenderSelectionScreen(this.ui.font);
    }
    this.genderSelection.reset();
  }

  private async ensureOakIntroSequence(): Promise<void> {
    if (!this.oakIntroSequence) {
      const font = this.ui.font;
      if (!font) {
        throw new Error("Oak intro sequence requires an initialized font renderer.");
      }
      this.oakIntroSequence = await OakIntroSequence.create(
        this.audioEngine,
        this.gameState,
        font
      );
    }
    this.oakIntroSequence?.setPlayerGender(this.getBootPlayerGender());
    this.oakIntroSequence?.setInstantMode(this.options.suppressBootAnimations === true);
  }

  private enterPlayerNameEntryState(): void {
    this.audioEngine.clearMapMusic();
    const promptText = "YOUR NAME?";
    if (!this.playerNameEntryScreen) {
      this.playerNameEntryScreen = new NameEntryScreen(
        this.ui as unknown as ScreenUI,
        promptText,
        this.audioEngine
      );
    }
    this.playerNameEntryScreen.reset({ prompt: promptText, maxNameLength: 7 });
    this.playerNameEntryScreen.fillName(String(this.gameState.sram.player_name ?? "").trim());
    this.currentState = "name_entry";
  }

  private startBootTransition(task: () => void | Promise<void>): void {
    if (this.bootTransitionTask) {
      return;
    }
    if (this.options.suppressBootAnimations) {
      try {
        const result = task();
        if (result && typeof (result as { then?: unknown }).then === "function") {
          const transition = (result as Promise<void>).catch((error) => {
            logger.error("[game] Boot transition failed", error);
          }) as Promise<void>;
          this.bootTransitionTask = transition;
          transition.finally(() => {
            if (this.bootTransitionTask === transition) {
              this.bootTransitionTask = null;
            }
          });
        }
      } catch (error) {
        logger.error("[game] Boot transition failed", error);
      }
      this.bootTransitionBlackout = false;
      return;
    }
    // Immediately black-out the screen so no stale state is visible during
    // the transition.  The flag stays on until the task resolves.
    this.bootTransitionBlackout = true;
    try {
      const result = task();
      if (result && typeof (result as { then?: unknown }).then === "function") {
        const transition = (result as Promise<void>).catch((error) => {
          logger.error("[game] Boot transition failed", error);
        }) as Promise<void>;
        this.bootTransitionTask = transition;
        transition.finally(() => {
          if (this.bootTransitionTask === transition) {
            this.bootTransitionTask = null;
            this.bootTransitionBlackout = false;
          }
        });
      } else {
        // Synchronous transition — lift blackout immediately.
        this.bootTransitionBlackout = false;
        const transition = Promise.resolve();
        this.bootTransitionTask = transition;
        transition.finally(() => {
          if (this.bootTransitionTask === transition) {
            this.bootTransitionTask = null;
          }
        });
        return;
      }
    } catch (error) {
      logger.error("[game] Boot transition failed", error);
      this.bootTransitionBlackout = false;
      return;
    }
  }

  private queueOverworldMusicAction(action: "start" | "restart"): void {
    this.pendingOverworldMusicAction = action;
  }

  private clearPendingOverworldMusicAction(): void {
    this.pendingOverworldMusicAction = null;
  }

  private flushPendingOverworldMusicAction(): void {
    if (!this.pendingOverworldMusicAction) {
      return;
    }
    const action = this.pendingOverworldMusicAction;
    this.pendingOverworldMusicAction = null;
    try {
      if (action === "restart") {
        this.overworld.restart_map_music();
      } else {
        this.overworld.start_map_music();
      }
      this.overworld.audio_controller?.update?.();
    } catch {
      // Map music unavailable for this frame; leave retries to later transitions.
    }
  }

  private async rebuildRuntimeForSaveState(
    gameState?: GameState,
    options: { suppressInitialMapEntryEffects?: boolean; suppressInitialMapMusic?: boolean } = {}
  ): Promise<void> {
    const nextState = gameState ?? this.gameState;
    if (!getMapMetadataByGroup(nextState.wram.wMapGroup, nextState.wram.wMapNumber)) {
      const spawnId = nextState.wram.wDefaultSpawnpoint ?? Spawn.HOME;
      try {
        applySpawn(nextState, spawnId as Spawn);
      } catch {
        applySpawn(nextState, Spawn.HOME);
      }
    }

    this.gameState = nextState;
    this.eventManager = new EventManager(this.gameState);
    this.eventManager.on<StartBattleEventPayload>("start_battle", this.startBattle.bind(this));
    this.eventManager.on("battle_complete", this.handleBattleCompleteAutosave.bind(this));
    this.eventManager.on("player_step", this.handlePlayerStepAutosave.bind(this));

    this.overworld = new Overworld(
      this.gameState,
      this.dataLoader,
      this.eventManager as any,
      this.tileset,
      this.audioEngine,
      this.ui,
      {
        suppressInitialMapEntryEffects: options.suppressInitialMapEntryEffects,
        suppressInitialMapMusic: options.suppressInitialMapMusic,
      }
    );
    this.whiteoutManager = new WhiteoutManager(this.gameState, this.overworld, this.eventManager);

    const menuOverworld = this.overworld as unknown as ConstructorParameters<typeof MenuState>[6];
    this.menuState = new MenuState(
      this.ui as any,
      this.gameState,
      this.audioEngine,
      this.tileset,
      this.dataLoader,
      this.overworld.script_runner ?? null,
      menuOverworld
    );

    this.overworld.reset_input_state();
    this.multiplayerBattle?.destroy();
    this.multiplayerBattle = null;
    this.battle = null;
    this.autosaveStepCounter = 0;
    set_game_state(this.battleUi, this.gameState);

    this.installBgMapSync();
    await this.overworld.init_assets();
  }

  private setBootPlayerGender(gender: PlayerGender): void {
    this.setPlayerGender(gender);
    if (this.currentState === "oak_intro") {
      this.oakIntroSequence?.setPlayerGender(gender);
    }
  }

  private getBootPlayerGender(): PlayerGender {
    return this.gameState.sram.player_gender === PlayerGender.FEMALE
      ? PlayerGender.FEMALE
      : PlayerGender.MALE;
  }

  private enterTitleState(): void {
    this.clearPendingOverworldMusicAction();
    this.audioEngine.clearMapMusic();
    this.currentState = "title";
    this.isOakIntroFinalSequence = false;
    this.introSequence = null;
    this.titleScreen?.startFromGameStart();
  }

  private async enterMainMenuState(): Promise<void> {
    this.clearPendingOverworldMusicAction();
    this.audioEngine.clearMapMusic();
    if (!this.mainMenu) {
      this.mainMenu = new MainMenu(
        this.ui as any,
        this.audioEngine,
        this.gameState,
        this.hasPersistedSaveData
      );
    }
    this.mainMenu.refresh(this.hasPersistedSaveData);
    this.mainMenu.startFadeIn();
    if (this.options.suppressBootAnimations) {
      this.mainMenu.skipFade();
    }
    this.isOakIntroFinalSequence = false;
    this.currentState = "main_menu";
  }

  private enterContinueState(): void {
    this.clearPendingOverworldMusicAction();
    this.audioEngine.clearMapMusic();
    this.ensureContinueScreen();
    this.isOakIntroFinalSequence = false;
    this.currentState = "continue";
  }

  private enterDeleteSaveState(): void {
    this.clearPendingOverworldMusicAction();
    this.audioEngine.clearMapMusic();
    if (!this.deleteSaveScreen) {
      this.deleteSaveScreen = new DeleteSaveScreen(this.ui as unknown as ScreenUI);
    }
    this.deleteSaveScreen.reset();
    this.isOakIntroFinalSequence = false;
    this.currentState = "delete_save";
  }

  private enterClockResetState(): void {
    this.clearPendingOverworldMusicAction();
    this.audioEngine.clearMapMusic();
    if (!this.clockResetScreen) {
      this.clockResetScreen = new ClockResetScreen(this.ui as unknown as ScreenUI, this.gameState);
    }
    this.clockResetScreen.reset();
    this.isOakIntroFinalSequence = false;
    this.currentState = "clock_reset";
  }

  private enterGenderState(): void {
    this.clearPendingOverworldMusicAction();
    this.audioEngine.clearMapMusic();
    this.ensureGenderSelection();
    this.genderSelection?.reset();
    this.isOakIntroFinalSequence = false;
    this.currentState = "gender";
  }

  private enterIntroSequenceState(): void {
    this.clearPendingOverworldMusicAction();
    this.audioEngine.clearMapMusic();
    this.audioEngine.playMusic("MUSIC_NONE", "intro");
    this.ensureIntroSequence();
    this.introSequence?.reset();
    this.isOakIntroFinalSequence = false;
    this.currentState = "intro";
  }

  private async enterOakIntroState(): Promise<void> {
    this.clearPendingOverworldMusicAction();
    this.audioEngine.clearMapMusic();
    await this.ensureOakIntroSequence();
    this.oakIntroSequence?.reset();
    this.isOakIntroFinalSequence = false;
    this.currentState = "oak_intro";
    this.setBootPlayerGender(this.getBootPlayerGender());
  }

  private requireOakIntroPlayerName(): string {
    const playerName = this.gameState.sram.player_name?.trim() ?? "";
    return playerName || DEFAULT_PLAYER_NAME;
  }

  private getBootScreenInputKey(event: { key?: string | number | null; code?: string | number | null }): string {
    const mapInput = (value: string | number | null | undefined): string | null => {
      if (value == null) {
        return null;
      }
      const button = mapKeyToButton(value);
      if (button === "a") {
        return "a";
      }
      if (button === "b") {
        return "b";
      }
      if (button === "start") {
        return "Enter";
      }
      if (button === "select") {
        return "Select";
      }
      const direction = mapKeyToDirection(value);
      if (direction === "up") {
        return "ArrowUp";
      }
      if (direction === "down") {
        return "ArrowDown";
      }
      if (direction === "left") {
        return "ArrowLeft";
      }
      if (direction === "right") {
        return "ArrowRight";
      }
      return typeof value === "string" ? value : null;
    };

    const mappedCode = mapInput(event.code);
    if (mappedCode) {
      return mappedCode;
    }
    const mappedKey = mapInput(event.key);
    if (mappedKey) {
      return mappedKey;
    }

    const raw = event.code ?? event.key;
    if (typeof raw === "string") {
      return raw;
    }
    if (typeof raw === "number") {
      const direction = mapKeyToDirection(raw);
      if (direction === "up") {
        return "ArrowUp";
      }
      if (direction === "down") {
        return "ArrowDown";
      }
      if (direction === "left") {
        return "ArrowLeft";
      }
      if (direction === "right") {
        return "ArrowRight";
      }
      const button = mapKeyToButton(raw);
      if (button === "a") return "a";
      if (button === "b") return "b";
      if (button === "start") return "Enter";
      if (button === "select") return "Select";
    }
    return "";
  }

  private getBootRenderContext(): CanvasRenderingContext2D {
    if (!this.bootRenderContext) {
      const ctx = this.ui.screen.canvas.getContext("2d");
      if (!ctx) {
        throw new Error("Boot screen rendering context missing.");
      }
      this.bootRenderContext = ctx as unknown as CanvasRenderingContext2D;
    }
    const bootCtx = this.bootRenderContext as CanvasRenderingContext2D & {
      setTransform?: (...args: [number, number, number, number, number, number]) => void;
      globalCompositeOperation?: string;
      imageSmoothingEnabled?: boolean;
    };
    bootCtx.setTransform?.(1, 0, 0, 1, 0, 0);
    bootCtx.globalAlpha = 1;
    if ("globalCompositeOperation" in bootCtx) {
      bootCtx.globalCompositeOperation = "source-over";
    }
    if ("imageSmoothingEnabled" in bootCtx) {
      bootCtx.imageSmoothingEnabled = false;
    }
    return this.bootRenderContext;
  }

  private async enterNewGameFlow(): Promise<void> {
    const nextState = createInitialGameState();
    nextState.wram.wDefaultSpawnpoint = this.gameState.wram.wDefaultSpawnpoint ?? Spawn.HOME;
    await this.rebuildRuntimeForSaveState(nextState, {
      suppressInitialMapEntryEffects: true,
      suppressInitialMapMusic: true,
    });
    this.enterGenderState();
  }

  private async transitionToOverworldState(nextState?: GameState): Promise<void> {
    const previousState = this.currentState;
    // Do NOT set currentState to "overworld" yet — the overworld assets are
    // not ready and rendering it would show a blank/stale map (the flicker).
    // The bootTransitionBlackout flag already ensures a black screen.

    try {
      await this.rebuildRuntimeForSaveState(nextState, {
        suppressInitialMapMusic: true,
      });
      this.audioEngine.clearMapMusic();
      this.audioEngine.stopMusic();
      // Only now that the overworld is fully rebuilt do we switch to it.
      this.currentState = "overworld";
      this.queueOverworldMusicAction("start");
    } catch (error) {
      this.currentState = previousState;
      throw error;
    }
  }

  private static async preload_core_assets(
    onProgress?: (completed: number, total: number, label?: string) => void
  ): Promise<void> {
    if (coreAssetsPreloadState === "ready") {
      return;
    }
    if (coreAssetsPreloadState === "loading" && coreAssetsPreloadPromise) {
      await coreAssetsPreloadPromise;
      return;
    }
    if (!coreAssetsPreloadPromise) {
      coreAssetsPreloadPromise = (async (): Promise<void> => {
        const { preload } = gameEngine.image;
        if (typeof preload !== "function") {
          coreAssetsPreloadState = "ready";
          return;
        }
        const spriteRoot = getAssetPath("gfx", "sprites");
        let spriteFiles: string[] = [];
        try {
          spriteFiles = fs.readdirSync(spriteRoot).filter((entry) => entry.endsWith(".png"));
        } catch {
          spriteFiles = [];
        }
        const spritePaths = new Set(BOOT_SPRITE_PATHS);
        let listAssetFilesBySuffixes:
          | ((suffixes: string[], options?: { prefixes?: string[] }) => string[])
          | null = null;
        if (typeof window !== "undefined") {
          const assetManifest = await import("@pokecrystal/core/core/asset-manifest");
          listAssetFilesBySuffixes = assetManifest.listAssetFilesBySuffixes;
          const manifestSpriteFiles = listAssetFilesBySuffixes(PRELOAD_SPRITE_SUFFIXES, {
            prefixes: PRELOAD_SPRITE_PREFIXES,
          });
          for (const spritePath of manifestSpriteFiles) {
            spritePaths.add(spritePath);
          }
        }
        for (const entry of spriteFiles) {
          spritePaths.add(getAssetPath("gfx", "sprites", entry));
        }
        for (const path of BOOT_PC_PATHS) {
          spritePaths.add(path);
        }
        for (const path of BOOT_BATTLE_PATHS) {
          spritePaths.add(path);
        }
        for (const path of BOOT_OVERWORLD_PATHS) {
          spritePaths.add(path);
        }
        for (const path of BOOT_NAMING_SCREEN_PATHS) {
          spritePaths.add(path);
        }
        for (const path of BOOT_INTRO_PATHS) {
          spritePaths.add(path);
        }
        for (const path of BOOT_TRAINER_CARD_PATHS) {
          spritePaths.add(path);
        }
        const tilesetRoot = getAssetPath("gfx", "tilesets");
        let tilesetFiles: string[] = [];
        try {
          tilesetFiles = fs.readdirSync(tilesetRoot).filter((entry) => entry.endsWith(".png"));
        } catch {
          tilesetFiles = [];
        }
        for (const entry of tilesetFiles) {
          spritePaths.add(getAssetPath("gfx", "tilesets", entry));
        }
        const imagePaths = Array.from(spritePaths);
        const dataPrefetchMode = resolveCoreDataPrefetchMode();
        let dataFiles: string[] = [];
        if (typeof window !== "undefined" && dataPrefetchMode !== "none") {
          const listFromManifest =
            listAssetFilesBySuffixes ?? (await import("@pokecrystal/core/core/asset-manifest")).listAssetFilesBySuffixes;
          dataFiles = Array.from(
            new Set([
              ...listFromManifest(PRELOAD_DATA_SUFFIXES, {
                prefixes: PRELOAD_DATA_PREFIXES,
              }),
              ...listFromManifest(PRELOAD_BATTLE_ANIM_SUFFIXES, {
                prefixes: PRELOAD_BATTLE_ANIM_PREFIXES,
              }),
            ])
          );
        }
        const totalAssets =
          imagePaths.length +
          (dataPrefetchMode === "blocking" ? dataFiles.length : 0) +
          UNOWN_PUZZLE_ASSET_TOTAL;
        if (totalAssets > 0) {
          onProgress?.(0, totalAssets);
        }
        const progressState = {
          completed: 0,
          total: totalAssets,
        };
        const loads = imagePaths.map((path) =>
          preload(path)
            .catch(() => null)
            .finally(() => {
              progressState.completed += 1;
              onProgress?.(progressState.completed, totalAssets, path);
            })
        );
        await Promise.all(loads);

        if (typeof window !== "undefined" && dataFiles.length > 0) {
          const warmDataAssets = async (trackProgress: boolean): Promise<void> => {
            const { prefetchFiles } = await import("@/shims/fs-browser");
            await prefetchFiles(dataFiles, {
              ignoreMissing: true,
              concurrency:
                dataPrefetchMode === "blocking"
                  ? BLOCKING_PREFETCH_CONCURRENCY
                  : DEFERRED_PREFETCH_CONCURRENCY,
              onProgress: trackProgress
                ? (dataCompleted, _dataTotal, path) => {
                    progressState.completed = imagePaths.length + dataCompleted;
                    onProgress?.(progressState.completed, totalAssets, path);
                  }
                : undefined,
            });
          };
          if (dataPrefetchMode === "blocking") {
            await warmDataAssets(true);
          } else {
            void warmDataAssets(false).catch((error) => {
              logger.warn("[game] Background core data prefetch failed", error);
            });
          }
        }
        await preloadUnownPuzzleAssets(onProgress, progressState);
      })();
    }
    try {
      coreAssetsPreloadState = "loading";
      await coreAssetsPreloadPromise;
      coreAssetsPreloadState = "ready";
    } finally {
      if (coreAssetsPreloadState !== "ready") {
        coreAssetsPreloadState = "idle";
      }
      coreAssetsPreloadPromise = null;
    }
    return;
  }

  private static async prepare_ui(ui: BaseUI): Promise<void> {
    assertAsmUiInvariants(ui, "Game.prepare_ui");

    const uiAny = ui as BaseUI & {
      tile_size?: number;
      tileSize?: number;
      font?: TextboxFontRenderer;
    };

    if (!uiAny.tile_size) {
      uiAny.tile_size = TILE_SIZE;
    }
    if (!uiAny.tileSize) {
      uiAny.tileSize = uiAny.tile_size;
    }

    const existingTiles = uiAny.font?.font_tiles ?? uiAny.font?.fontTiles ?? null;
    const hasExistingRendererBindings =
      Boolean(uiAny.font?.render_text || uiAny.font?.renderText) &&
      Boolean(uiAny.font?.get_char_tile || uiAny.font?.getCharTile);
    if (existingTiles && Object.keys(existingTiles).length > 0 && hasExistingRendererBindings) {
      return;
    }

    const renderer = new TextRenderer();
    await renderer.load();
    if (!uiAny.font) {
      uiAny.font = {};
    }
    if (!uiAny.font.font_tiles) {
      uiAny.font.font_tiles = renderer.font_tiles;
    }
    if (!uiAny.font.fontTiles) {
      uiAny.font.fontTiles = renderer.fontTiles;
    }
    if (!uiAny.font.render_text) {
      uiAny.font.render_text = renderer.render_text.bind(renderer) as any;
    }
    if (!uiAny.font.renderText) {
      uiAny.font.renderText = renderer.renderText.bind(renderer) as any;
    }
    if (!uiAny.font.get_char_tile) {
      uiAny.font.get_char_tile = renderer.getCharTile.bind(renderer) as any;
    }
    if (!uiAny.font.getCharTile) {
      uiAny.font.getCharTile = renderer.getCharTile.bind(renderer) as any;
    }
    if (!uiAny.font.paletteVariants) {
      uiAny.font.paletteVariants = renderer.paletteVariants.bind(renderer);
    }

    const frameRoot = getAssetPath("gfx", "frames");
    let frameIds: number[] = FALLBACK_FRAME_IDS;
    try {
      const entries = fs.readdirSync(frameRoot);
      const discoveredFrameIds: number[] = [];
      for (const entry of entries) {
        if (!entry.endsWith(SPRITE_EXT)) {
          continue;
        }
        const value = Number(entry.substring(0, entry.length - SPRITE_EXT.length));
        if (value > 0 && Number.isFinite(value)) {
          discoveredFrameIds.push(value);
        }
      }
      if (discoveredFrameIds.length > 0) {
        frameIds = discoveredFrameIds;
      }
    } catch {
      frameIds = FALLBACK_FRAME_IDS;
    }
    if (typeof uiAny.preloadWindowFrames === "function") {
      await uiAny.preloadWindowFrames(frameIds);
    }
  }

  constructor(private ui: BaseUI, private readonly options: GameOptions = {}, initialState?: GameState) {
    this.autosaveSlot = options.autosaveSlot?.trim() || AUTOSAVE_SLOT;
    this.bootSaveSlot = resolveBootSaveSlot(options.bootSaveSlot ?? options.loadSlot);
    this.bootSequenceMode = options.initialState
      ? options.initialState === "intro"
        ? "intro"
        : "overworld"
      : options.playIntro
        ? "intro"
        : options.newGame
          ? "title"
          : "overworld";
    if (options.initialState) {
      this.currentState = options.initialState;
    } else if (this.bootSequenceMode === "intro" || this.bootSequenceMode === "title") {
      this.currentState = this.bootSequenceMode;
    }

    const shouldStartFresh =
      this.bootSequenceMode === "title" &&
      !options.initialState &&
      !options.playIntro &&
      !initialState;
    if (shouldStartFresh && options.initialSpawnId !== undefined) {
      const seeded = createInitialGameState();
      seeded.wram.wDefaultSpawnpoint = options.initialSpawnId;
      this.gameState = seeded;
    } else if (shouldStartFresh) {
      this.gameState = createInitialGameState();
    } else if (!initialState && options.initialSpawnId !== undefined) {
      const seeded = createInitialGameState();
      seeded.wram.wDefaultSpawnpoint = options.initialSpawnId;
      this.gameState = seeded;
    } else {
      this.gameState = initialState ?? createInitialGameState();
    }
    this.ui.eventQueue = this.eventQueue;
    this.installBgMapSync();
    if (!getMapMetadataByGroup(this.gameState.wram.wMapGroup, this.gameState.wram.wMapNumber)) {
      const spawnId = this.gameState.wram.wDefaultSpawnpoint ?? Spawn.HOME;
      try {
        applySpawn(this.gameState, spawnId as Spawn);
      } catch {
        applySpawn(this.gameState, Spawn.HOME);
      }
    }
    this.eventManager = new EventManager(this.gameState);
    this.dataLoader = new DataLoader();
    this.dataLoader.Tileset = OverworldTileset;
    this.audioEngine = new AudioEngine({
      masterVolume: options.masterVolume ?? 0.5,
      muted: options.muted ?? false,
    });
    const timeOfDay = String(this.gameState.wram.time_of_day ?? "day");
    this.tileset = new OverworldTileset("johto", timeOfDay);

    const shouldBenchmarkFrames =
      Boolean(options.benchmarking?.enabled) ||
      isDebugEnabled("flare_plot") ||
      isDebugEnabled("benchmark");
    this.benchmark =
      shouldBenchmarkFrames
        ? new GameBenchmark({ ...options.benchmarking, enabled: true }, () => this._resolveTimestampMs())
        : null;

    this.overworld = new Overworld(
      this.gameState,
      this.dataLoader,
      this.eventManager as any,
      this.tileset,
      this.audioEngine,
      this.ui,
      {
        suppressInitialMapEntryEffects: this.currentState !== "overworld",
        suppressInitialMapMusic: true,
      }
    );
    this.whiteoutManager = new WhiteoutManager(this.gameState, this.overworld, this.eventManager);

    const battleUi = this.ui;
    assertBattleUI(battleUi);
    this.battleUi = createBattleUI(battleUi, {
      layout: BattleUILayoutFactory.fromAsmDefaults(),
      game_state: this.gameState,
      data_loader: this.dataLoader,
      load_data: false,
    });
    set_audio_engine(this.battleUi, this.audioEngine);

    const menuUi = this.ui;
    assertMenuUI(menuUi);
    const menuOverworld = this.overworld as unknown as ConstructorParameters<typeof MenuState>[6];
    this.menuState = new MenuState(
      menuUi,
      this.gameState,
      this.audioEngine,
      this.tileset,
      this.dataLoader,
      this.overworld.script_runner ?? null,
      menuOverworld,
    );

    this.eventManager.on<StartBattleEventPayload>("start_battle", this.startBattle.bind(this));
    this.eventManager.on("battle_complete", this.handleBattleCompleteAutosave.bind(this));
    this.eventManager.on("player_step", this.handlePlayerStepAutosave.bind(this));
    if (this.currentState === "overworld") {
      this.queueOverworldMusicAction("start");
    }
  }

  private loopCircuitBreaker = createCircuitBreaker(10000, "game.gameLoop");

  public async init(): Promise<void> {
    if (!this.initPromise) {
      this.initPromise = this.overworld.init_assets();
    }
    await this.initPromise;
  }

  public getGameState(): GameState {
    return this.gameState;
  }

  public getMapName(): string {
    return this.overworld.current_map?.name ?? "Unknown";
  }

  public getCurrentMapName(): string {
    return this.getMapName();
  }

  public getPartyPokemon(): Pokemon[] {
    return (this.gameState.sram.party?.pokemon ?? [])
      .filter((pokemon): pokemon is NonNullable<typeof pokemon> => Boolean(pokemon))
      .map((pokemon) => toPokemon(pokemon as Pokemon));
  }

  public getFirstPartyPokemon(): { index: number; pokemon: Pokemon } | null {
    const party = this.gameState.sram.party?.pokemon ?? [];
    for (let index = 0; index < party.length; index += 1) {
      const pokemon = party[index];
      if (pokemon) {
        return { index, pokemon: toPokemon(pokemon as Pokemon) };
      }
    }
    return null;
  }

  public replacePartyPokemon(index: number, pokemon: Pokemon): void {
    const party = this.gameState.sram.party?.pokemon;
    if (!party) {
      throw new Error("Party is not initialized.");
    }
    if (!Number.isInteger(index) || index < 0 || index >= party.length) {
      throw new Error(`Party index out of range: ${index}`);
    }
    const parsed = PokemonSchema.safeParse(pokemon);
    if (!parsed.success) {
      throw new Error("Replacement Pokemon is invalid.");
    }
    party[index] = toPokemon(parsed.data);
  }

  public startMultiplayerBattle(remoteParty: Pokemon[]): void {
    const localParty = this.getPartyPokemon();
    if (!localParty.length) {
      throw new Error("Cannot start multiplayer battle without a local party.");
    }
    if (!remoteParty.length) {
      throw new Error("Cannot start multiplayer battle without a remote party.");
    }
    this.postEvent(
      new StartBattleEvent({
        player_pokemon: localParty[0],
        enemy_pokemon: remoteParty[0],
        player_party: localParty,
        enemy_party: remoteParty,
      })
    );
  }

  public isMenuOpen(): boolean {
    const runner = this.overworld.script_runner ?? null;
    const runnerBusy =
      runner && typeof runner.is_busy === "boolean"
        ? runner.is_busy
        : runner && typeof (runner as { is_busy?: () => boolean }).is_busy === "function"
          ? (runner as { is_busy?: () => boolean }).is_busy?.() ?? false
          : runner && runner.state !== undefined
            ? runner.state !== 0 && runner.state !== "idle"
            : Boolean(
                runner &&
                  ((runner._script_stack?.length ?? 0) > 0 ||
                    (runner._awaiting_resume ?? 0) > 0 ||
                    runner.stop_execution ||
                    runner.stopExecution)
              );
    return this.currentState === "menu" || runnerBusy || false;
  }

  public isBattleActive(): boolean {
    return this.currentState === "battle";
  }

  public start() {
    if (!this.initPromise) {
      throw new Error("Game assets not initialized; call await Game.create(...) or await game.init() before start.");
    }
    this._clearScheduledLoop();
    this.overworld.reset_input_state();
    this.eventQueue.length = 0;
    this._installFatalHandlers();
    this._resetFrameTiming();
    this.gameLoopBound();
  }

  public destroy(): void {
    this.quitRequested = true;
    this._clearScheduledLoop();
    this.audioEngine.dispose();
    this.multiplayerBattle?.destroy();
    this.multiplayerBattle = null;
    this.multiplayerBattleTransport = null;
    this.multiplayerBattleCompleteCallback = null;
  }

  public unlockAudio(): void {
    this.audioEngine.unlock();
  }

  public setAudioMuted(muted: boolean): void {
    this.audioEngine.setMuted(muted);
  }

  public setMusicMuted(muted: boolean): void {
    this.audioEngine.setMusicMutedByController(muted);
  }

  public getAudioPlaybackSnapshot(): AudioPlaybackSnapshot {
    return this.audioEngine.getPlaybackSnapshot();
  }

  public tick(): void {
    if (this.fatalError) {
      return;
    }

    const benchmark = this.benchmark;
    const tickStart = benchmark ? this._resolveTimestampMs() : 0;
    const previousQueue = gameEngine.event.getActiveQueue();
    const shouldSwapEventQueue = previousQueue !== this.eventQueue;
    if (shouldSwapEventQueue) {
      gameEngine.event.setActiveQueue(this.eventQueue);
    }
    try {
      const overworldFatal = this.overworld.fatal_error ?? null;
      if (overworldFatal) {
        throw overworldFatal;
      }
      resetJoypadFrame(this.gameState.hram.joypad);
      this.gameState.frame_counter += 1;
      benchmark?.beginFrame(this.gameState.frame_counter, tickStart, this.currentState);

      if (benchmark) {
        const inputStart = this._resolveTimestampMs();
        this.handleInput();
        benchmark.recordPhase("handleInput", this._resolveTimestampMs() - inputStart);

        const updateStart = this._resolveTimestampMs();
        this.update();
        benchmark.recordPhase("update", this._resolveTimestampMs() - updateStart);

        const drawStart = this._resolveTimestampMs();
        this.draw();
        benchmark.recordPhase("draw", this._resolveTimestampMs() - drawStart);
      } else {
        this.handleInput();
        this.update();
        this.draw();
      }
    } finally {
      if (benchmark) {
        benchmark.endFrame(this._resolveTimestampMs() - tickStart);
      }
      if (shouldSwapEventQueue) {
        gameEngine.event.setActiveQueue(previousQueue);
      }
    }
  }

  public getState(): GameStateName {
    return this.currentState;
  }

  public getOverworld(): Overworld {
    return this.overworld;
  }

  public getMenuState(): MenuState {
    return this.menuState;
  }

  public getBattle(): Battle | null {
    return this.battle;
  }

  private async resolveActiveBootSaveSlot(): Promise<string | null> {
    for (const slot of resolveBootSaveCandidates(this.bootSaveSlot)) {
      try {
        if (await hasSaveGame(slot)) {
          return slot;
        }
      } catch (error) {
        logger.warn(`[game] Unable to probe boot save slot ${slot}`, error);
      }
    }
    return null;
  }

  public async debugJumpToScene(scene: GameDebugScene): Promise<void> {
    switch (scene) {
      case "intro":
        await this.ensureTitleScreen();
        this.enterIntroSequenceState();
        this.titleScreen?.startFromGameStart();
        return;
      case "title":
        await this.ensureTitleScreen();
        this.enterTitleState();
        return;
      case "main_menu":
        await this.ensureTitleScreen();
        await this.enterMainMenuState();
        return;
      case "continue":
        await this.ensureTitleScreen();
        this.enterContinueState();
        return;
      case "delete_save":
        await this.ensureTitleScreen();
        this.enterDeleteSaveState();
        return;
      case "clock_reset":
        await this.ensureTitleScreen();
        this.enterClockResetState();
        return;
      case "gender":
        await this.rebuildRuntimeForSaveState(this.buildDebugState(), {
          suppressInitialMapEntryEffects: true,
          suppressInitialMapMusic: true,
        });
        this.enterGenderState();
        return;
      case "oak_intro":
        await this.rebuildRuntimeForSaveState(this.buildDebugState(), {
          suppressInitialMapEntryEffects: true,
          suppressInitialMapMusic: true,
        });
        await this.enterOakIntroState();
        return;
      case "new_game":
        await this.enterNewGameFlow();
        return;
      case "overworld":
        await this.transitionToOverworldState(this.buildDebugState());
        return;
      default:
        throw new Error(`Unsupported debug scene '${String(scene)}'.`);
    }
  }

  public async debugJumpToSpawn(spawn: Spawn): Promise<void> {
    const nextState = this.buildDebugState();
    applySpawn(nextState, spawn);
    await this.transitionToOverworldState(nextState);
  }

  public async debugSaveToSlot(
    slot: string,
    options?: { withHistory?: boolean }
  ): Promise<boolean> {
    if (options?.withHistory) {
      return saveGameWithHistory(this.gameState, slot, MANUAL_SAVE_HISTORY_SLOTS);
    }
    return saveGame(this.gameState, slot);
  }

  public async debugDeleteSaveSlot(slot: string): Promise<boolean> {
    const deleted = await deleteSaveGame(slot);
    if (deleted && slot === this.bootSaveSlot) {
      this.hasPersistedSaveData = false;
    }
    return deleted;
  }

  public async debugHasSaveSlot(slot: string): Promise<boolean> {
    return hasSaveGame(slot);
  }

  public async debugTriggerAutosave(
    reason: "battle_complete" | "player_steps" = "battle_complete",
    count: number = AUTOSAVE_STEP_THRESHOLD
  ): Promise<void> {
    if (reason === "player_steps") {
      await this.handlePlayerStepAutosave(new Event("player_step", { count }));
      return;
    }
    await this.handleBattleCompleteAutosave(new Event("battle_complete", { result: 0 }));
  }

  public getDebugStatus(): GameDebugStatus {
    const state = this.gameState;
    const overworld = this.overworld as Overworld & {
      dialogue?: {
        active?: boolean;
        visible?: boolean;
        waiting_for_input?: boolean;
        pending_waits?: number;
      } | null;
      script_runner?: {
        is_busy?: boolean;
        _script_stack?: unknown[];
        _awaiting_resume?: number;
      } | null;
      _movement_lock_count?: number;
      _text_lock_active?: boolean;
      input_capture_active?: boolean;
      script_tasks_active?: () => boolean;
    };
    const menuState = this.menuState as unknown as {
      dialogueVisible?: boolean;
      dialogue?: {
        visible_text?: string;
        is_complete?: () => boolean;
        has_more_pages?: () => boolean;
      };
    };
    const battleUi = this.battleUi as BattleUIState & {
      waiting_for_input?: boolean;
      dialogue?: {
        queue?: unknown[];
        pending_waits?: number;
        dialogue?: {
          visible_text?: string;
          is_complete?: () => boolean;
        };
      };
      yes_no_prompt?: {
        active?: boolean;
        pending_activation?: boolean;
      };
    };

    const overworldPromptPending = Boolean(overworld.dialogue?.waiting_for_input);
    const overworldYesNoPromptOpen = Boolean(
      (overworld.dialogue as { _yes_no_prompt?: unknown } | null | undefined)?._yes_no_prompt
    );
    const overworldDialogVisible =
      Boolean(overworld.dialogue?.active) ||
      Boolean(overworld.dialogue?.visible) ||
      Number(overworld.dialogue?.pending_waits ?? 0) > 0;
    const menuDialogVisible =
      this.currentState === "menu" &&
      (Boolean(menuState.dialogueVisible) || Boolean(menuState.dialogue?.visible_text));
    const battlePromptPending =
      Boolean(battleUi.waiting_for_input) ||
      Boolean(battleUi.yes_no_prompt?.active) ||
      Boolean(battleUi.yes_no_prompt?.pending_activation);
    const battleDialogVisible =
      Boolean(battleUi.dialogue?.dialogue?.visible_text) ||
      Number(battleUi.dialogue?.pending_waits ?? 0) > 0 ||
      (battleUi.dialogue?.queue?.length ?? 0) > 0 ||
      !(battleUi.dialogue?.dialogue?.is_complete?.() ?? true);
    const promptPending =
      this.currentStateHasBootIntro() ||
      overworldYesNoPromptOpen ||
      (this.currentState === "menu" && menuDialogVisible) ||
      (this.currentState === "battle" && battlePromptPending);
    const textAdvancePending =
      !this.currentStateHasBootIntro() &&
      this.currentState !== "battle" &&
      overworldPromptPending &&
      !overworldYesNoPromptOpen;
    const inDialog =
      this.currentStateHasBootIntro() ||
      overworldDialogVisible ||
      menuDialogVisible ||
      (this.currentState === "battle" && battleDialogVisible);
    const inMenu = this.currentState === "menu" || this.currentStateHasBootIntro();
    const scriptBusy =
      Boolean(overworld.script_tasks_active?.()) ||
      Boolean(overworld.script_runner?.is_busy) ||
      (overworld.script_runner?._script_stack?.length ?? 0) > 0 ||
      Number(overworld.script_runner?._awaiting_resume ?? 0) > 0;
    const movementLocked =
      Number(overworld._movement_lock_count ?? 0) > 0 ||
      Boolean(overworld._text_lock_active) ||
      Boolean(overworld.input_capture_active);
    const canMove =
      this.currentState === "overworld" &&
      !promptPending &&
      !scriptBusy &&
      !movementLocked &&
      !this.isBattleActive();

    let promptReason: string | null = null;
    if (this.currentStateHasBootIntro()) {
      promptReason = this.currentState;
    } else if (this.currentState === "battle" && battlePromptPending) {
      promptReason = "battle_prompt";
    } else if (this.currentState === "menu" && menuDialogVisible) {
      promptReason = "menu_dialogue";
    } else if (overworldYesNoPromptOpen) {
      promptReason = "overworld_prompt";
    } else if (scriptBusy) {
      promptReason = "script_runner";
    }

    return {
      mode: this.currentState,
      mapName: this.getMapName(),
      mapGroup: state.wram.wMapGroup,
      mapNumber: state.wram.wMapNumber,
      coords: {
        x: state.wram.wXCoord,
        y: state.wram.wYCoord,
      },
      prompt_pending: promptPending,
      text_advance_pending: textAdvancePending,
      prompt_reason: promptReason,
      in_dialog: inDialog,
      in_menu: inMenu,
      in_battle: this.isBattleActive(),
      movement_locked: movementLocked,
      script_busy: scriptBusy,
      can_move: canMove,
      current_spawn:
        typeof state.wram.wDefaultSpawnpoint === "number" ? state.wram.wDefaultSpawnpoint : null,
      oak_intro:
        this.currentState === "oak_intro"
          ? this.oakIntroSequence?.getDebugState() ?? null
          : null,
      intro:
        this.currentState === "intro"
          ? this.introSequence?.getDebugState() ?? null
          : null,
      name_entry:
        this.currentState === "name_entry"
          ? {
              finished: Boolean(this.playerNameEntryScreen?.finished),
              name: String(this.playerNameEntryScreen?.name ?? ""),
            }
          : null,
    };
  }

  /**
   * Configure a transport for wrapping the next non-trainer battle in MultiplayerBattle.
   *
   * Ownership: the caller owns the transport lifecycle; Game will only subscribe/unsubscribe.
   */
  public setMultiplayerBattleTransport(transport: BattleSyncTransport, options: { isHost: boolean }): void {
    this.multiplayerBattleTransport = transport;
    this.multiplayerIsHost = Boolean(options.isHost);
  }

  public clearMultiplayerBattleTransport(): void {
    if (this.multiplayerBattle) {
      this.multiplayerBattle.destroy();
      this.multiplayerBattle = null;
    }
    this.multiplayerBattleTransport = null;
    this.multiplayerIsHost = false;
  }

  public onMultiplayerBattleComplete(
    callback: ((result: MultiplayerBattleCompleteResult) => void) | null
  ): void {
    this.multiplayerBattleCompleteCallback = callback;
  }

  public setOverworldRemotePlayers(players: RemoteOverworldPlayer[]): void {
    this.overworld?.set_multiplayer_remote_players?.(players);
  }

  public clearOverworldRemotePlayers(): void {
    this.setOverworldRemotePlayers([]);
  }

  public setOverworldRemoteRenderEnabled(enabled: boolean): void {
    this.overworld?.set_multiplayer_remote_render_enabled?.(enabled);
  }

  public setOverworldRemoteCrowdView(enabled: boolean): void {
    this.overworld?.set_multiplayer_remote_crowd_view?.(enabled);
  }

  public getBattleUi(): BattleUIState {
    return this.battleUi;
  }

  public setPlayerGender(gender: PlayerGender): void {
    const normalized = gender === PlayerGender.FEMALE ? PlayerGender.FEMALE : PlayerGender.MALE;
    this.gameState.sram.player_gender = normalized;
    this.gameState.wram.player_gender = normalized;
    this.gameState.wram.wPlayerGender = normalized;
    this.genderSelection?.setSelectedGender(normalized);
    this.oakIntroSequence?.setPlayerGender(normalized);
    this.overworld?.refresh_player_sprite?.({ reload_standing: true, reload_walking: true });
  }

  public setPlayerName(name: string): void {
    const trimmed = String(name ?? "").trim();
    const normalized = trimmed.length > 0 ? trimmed.slice(0, 10) : DEFAULT_PLAYER_NAME;
    this.gameState.sram.player_name = normalized;
    if (this.currentState === "name_entry") {
      this.playerNameEntryScreen?.fillName(normalized);
    }
  }

  public getBenchmark(): GameBenchmark | null {
    return this.benchmark;
  }

  public clearBenchmark(): void {
    this.benchmark?.clear();
  }

  public setTimeOfDay(timeOfDay: TimeOfDay): void {
    const timeSystem = new TimeSystem(this.gameState);
    const { minute, second } = this._resolveGameClock();
    const targetHour = TIME_OF_DAY_HOURS[timeOfDay] ?? DAY_HOUR;
    const day = this.gameState.sram.day_of_week ?? 0;
    timeSystem.setManualTime({ day, hour: targetHour, minute, second });
  }

  public setDayOfWeek(day: number): void {
    const timeSystem = new TimeSystem(this.gameState);
    const { hour, minute, second } = this._resolveGameClock();
    timeSystem.setManualTime({ day, hour, minute, second });
  }

  public postEvent(event: InstanceType<typeof gameEngine.event.Event>): void {
    gameEngine.event.post(event, this.eventQueue);
  }

  private buildDebugState(): GameState {
    const nextState = createInitialGameState();
    nextState.sram.player_name = this.gameState.sram.player_name;
    nextState.sram.player_gender = this.gameState.sram.player_gender;
    nextState.sram.day_of_week = this.gameState.sram.day_of_week;
    nextState.sram.options = { ...this.gameState.sram.options };
    nextState.wram.time_of_day = this.gameState.wram.time_of_day;
    nextState.wram.player_gender = this.gameState.wram.player_gender;
    nextState.wram.wPlayerGender = this.gameState.wram.wPlayerGender;
    nextState.wram.instant_mode = this.gameState.wram.instant_mode;
    nextState.wram.wDefaultSpawnpoint =
      this.gameState.wram.wDefaultSpawnpoint ?? Spawn.NEW_BARK;
    return nextState;
  }

  private startBattle(event: Event<StartBattleEventPayload>) {
    this.overworld.reset_input_state();
    this.dataLoader.ensure_battle_data();
    const payload = event.data;
    const playerPokemon = payload.player_pokemon ?? payload.playerPokemon;
    if (!playerPokemon) {
      throw new Error("StartBattleEvent missing player Pokemon");
    }
    const enemyPokemon = payload.enemy_pokemon ?? payload.enemyPokemon;
    if (!enemyPokemon) {
      throw new Error("StartBattleEvent missing enemy Pokemon");
    }
    const ensurePokemon = (pokemon: Pokemon): Pokemon => {
      if (typeof pokemon._calculateStat === "function") {
        return pokemon;
      }
      const parsed = PokemonSchema.safeParse(pokemon);
      if (!parsed.success) {
        throw new Error("StartBattleEvent includes invalid Pokemon data.");
      }
      Object.assign(pokemon as Record<string, unknown>, parsed.data);
      const decorated = toPokemon(pokemon as Pokemon);
      if (decorated !== pokemon) {
        Object.assign(pokemon as Record<string, unknown>, decorated);
      }
      return pokemon as Pokemon;
    };
    const playerParty = payload.player_party ?? payload.playerParty ?? [playerPokemon];
    const enemyParty = payload.enemy_party ?? payload.enemyParty ?? [enemyPokemon];
    const normalizedPlayerPokemon = ensurePokemon(playerPokemon);
    const normalizedEnemyPokemon = ensurePokemon(enemyPokemon);
    const normalizedPlayerParty = playerParty.map((pokemon) =>
      pokemon === playerPokemon ? normalizedPlayerPokemon : ensurePokemon(pokemon)
    );
    const normalizedEnemyParty = enemyParty.map((pokemon) =>
      pokemon === enemyPokemon ? normalizedEnemyPokemon : ensurePokemon(pokemon)
    );
    if (!this._hasUsablePartyMember(normalizedPlayerParty)) {
      this.gameState.wram.battle_result = 1;
      this.eventManager.dispatch(new Event("battle_complete", { result: 1 }));
      return;
    }
    const trainer = payload.trainer ?? undefined;
    const trainerId = payload.trainer_id ?? payload.trainerId ?? undefined;
    const trainerReward = payload.trainer_reward ?? payload.trainerReward ?? 0;
    const autoInput = payload.auto_input ?? payload.autoInput;
    // ASM: engine/battle@pokecrystal/core.asm::BattleIntro -> PlayBattleMusic.
    const battleMusic = determineBattleMusic(this.gameState);
    this.audioEngine.playMusic(battleMusic, "battle");

    if (trainer) {
      this.battle = new TrainerBattle(
        normalizedPlayerPokemon,
        trainer,
        this.gameState,
        this.eventManager,
        this.battleUi,
        this.dataLoader.moveData,
        this.audioEngine,
        normalizedPlayerParty,
        normalizedEnemyParty,
        this.overworld,
        trainerId,
        trainerReward,
        autoInput
      );
    } else {
      this.battle = new Battle(
        normalizedPlayerPokemon,
        normalizedEnemyPokemon,
        this.gameState,
        this.eventManager,
        this.battleUi,
        this.dataLoader.moveData,
        this.audioEngine,
        trainer,
        normalizedPlayerParty,
        normalizedEnemyParty,
        this.overworld,
        trainerId,
        trainerReward,
        autoInput
      );
    }

    const multiplayerTransport = this.multiplayerBattleTransport;
    const multiplayerIsHost = this.multiplayerIsHost;
    if (multiplayerTransport) {
      this.multiplayerBattleTransport = null;
      this.multiplayerIsHost = false;
      if (trainer) {
        throw new Error("[game] MultiplayerBattle does not support TrainerBattle payloads yet.");
      }
      // Wrap the battle update loop to synchronize RNG + actions via transport.
      this.multiplayerBattle?.destroy();
      this.multiplayerBattle = new MultiplayerBattle({
        battle: this.battle,
        transport: multiplayerTransport,
        isHost: multiplayerIsHost,
        gameState: this.gameState,
      });
      void this.multiplayerBattle.initRng().catch((error) => {
        console.warn("[game] MultiplayerBattle RNG init failed.", error);
      });
    }

    begin_battle(this.battleUi);
    this.currentState = "battle";
  }

  private _hasUsablePartyMember(party: Pokemon[]): boolean {
    return party.some(
      (pokemon) =>
        pokemon.hp > 0 &&
        (pokemon.species.id.toUpperCase() !== "EGG") &&
        (pokemon.nickname ?? "").toUpperCase() !== "EGG"
    );
  }

  private _resolveGameClock(): { hour: number; minute: number; second: number } {
    const { game_time_hours, game_time_minutes, game_time_seconds } = this.gameState.sram;
    return {
      hour: Number.isFinite(game_time_hours) ? game_time_hours : 0,
      minute: Number.isFinite(game_time_minutes) ? game_time_minutes : 0,
      second: Number.isFinite(game_time_seconds) ? game_time_seconds : 0,
    };
  }

  private update() {
    // During a boot transition blackout, freeze all game logic so no stale
    // state or partially-initialized state advances.  Audio still updates
    // below so music transitions are smooth.
    if (this.bootTransitionBlackout) {
      this.audioEngine.update();
      return;
    }
    const isUnownPuzzleActive =
      (this.gameState.wram.wUnownState ?? 0) !== 0 ||
      getUnownOverlayLockDepth(this.gameState) > 0;
    // ASM parity: intro flow follows engine/menus/intro_menu.asm, including:
    // CrystalIntro -> StartTitleScreen -> TitleScreen/continue/delete/clock/gender/oak intro.
    this.tickBgMapSync();
    switch (this.currentState) {
      case "intro":
        if (this.introSequence?.update()) {
          this.startBootTransition(() => {
            this.enterTitleState();
          });
        }
        break;
  case "title":
        this.titleScreen?.update();
        const titleAction = this.titleScreen?.popAction();
        if (titleAction) {
          this.startBootTransition(() => {
            if (titleAction === TitleScreenOption.MAIN_MENU) {
              return this.enterMainMenuState();
            } else if (titleAction === TitleScreenOption.DELETE_SAVE_DATA) {
              this.enterDeleteSaveState();
            } else if (titleAction === TitleScreenOption.RESTART) {
              this.enterIntroSequenceState();
            } else if (titleAction === TitleScreenOption.RESET_CLOCK) {
              this.enterClockResetState();
            } else {
              this.enterTitleState();
            }
          });
        }
        break;
      case "main_menu":
        this.mainMenu?.update();
        break;
      case "continue":
        break;
      case "delete_save":
        break;
      case "clock_reset":
        break;
      case "gender":
        if (this.genderSelection?.update()) {
          const selectedGender = this.genderSelection.getSelectedGender();
          this.setPlayerGender(selectedGender);
          this.startBootTransition(async () => {
            await this.enterOakIntroState();
          });
        }
        break;
      case "oak_intro":
        if (!this.isOakIntroFinalSequence) {
          if (this.oakIntroSequence?.update()) {
            this.enterPlayerNameEntryState();
          }
        } else if (this.oakIntroSequence?.updateFinalEncouragement()) {
          this.startBootTransition(async () => {
            await this.transitionToOverworldState(this.gameState);
          });
        }
        break;
      case "name_entry":
        this.playerNameEntryScreen?.update();
        if (this.playerNameEntryScreen?.finished) {
          this.setPlayerName(this.playerNameEntryScreen.name);
          const playerName = this.requireOakIntroPlayerName();
          this.oakIntroSequence?.startFinalEncouragement(playerName);
          this.isOakIntroFinalSequence = true;
          this.currentState = "oak_intro";
        }
        break;
      case "overworld":
        if (isUnownPuzzleActive) {
          break;
        }
        this.overworld.update();
        break;
      case "battle":
        if (this.battle) {
          if (this.multiplayerBattle) {
            this.multiplayerBattle.update();
          } else {
            this.battle.update();
          }
          if (this.battle.isFinished()) {
            const completedMultiplayerBattle = this.multiplayerBattle;
            if (completedMultiplayerBattle) {
              this.multiplayerBattleCompleteCallback?.({
                result: Number(this.gameState.wram.battle_result ?? 0),
              });
            }
            this.battle.teardown();
            if (this.multiplayerBattle) {
              this.multiplayerBattle.destroy();
              this.multiplayerBattle = null;
            }
            end_battle(this.battleUi);
            this.overworld.reset_input_state();
            this.currentState = "overworld";
            this.queueOverworldMusicAction("restart");
          }
        }
        break;
      case "menu":
        // MenuState updates during draw to keep dialogue/timers in sync.
        break;
      // Other states...
    }
    this.whiteoutManager.update();
    this.audioEngine.update();
    this.eventManager.advanceFrame();
  }

  private installBgMapSync(): void {
    const runtime = this.gameState as GameState & BGMapSyncRuntime;
    if (!runtime.bg_map_sync) {
      runtime.bg_map_sync = { is_busy: false, remaining_frames: 0 };
    }
    if (typeof runtime.write_bg_map_with_wait === "function") {
      return;
    }
    runtime.write_bg_map_with_wait = (
      name,
      width,
      height,
      tiles,
      attrs,
      options = {}
    ) => {
      const manager = this._getVramManager();
      manager.writeBgRegion(name, width, height, tiles, attrs, {
        originX: options.origin_x ?? 0,
        originY: options.origin_y ?? 0,
      });
      const sync = runtime.bg_map_sync ?? { is_busy: false, remaining_frames: 0 };
      sync.is_busy = true;
      sync.remaining_frames = Math.max(1, sync.remaining_frames ?? 0);
      runtime.bg_map_sync = sync;
    };
  }

  private tickBgMapSync(): void {
    const runtime = this.gameState as GameState & BGMapSyncRuntime;
    const sync = runtime.bg_map_sync;
    if (!sync || !sync.is_busy) {
      return;
    }
    const remaining = Math.max(0, sync.remaining_frames ?? 0);
    if (remaining <= 0) {
      sync.is_busy = false;
      return;
    }
    sync.remaining_frames = remaining - 1;
    if (sync.remaining_frames <= 0) {
      sync.is_busy = false;
    }
  }

  private draw() {
    // During a boot transition blackout, render a black screen so nothing
    // from the previous or next state is visible.
    if (this.bootTransitionBlackout) {
      this.ui.clearScreen([0, 0, 0]);
      this.ui.update();
      return;
    }
    const isUnownPuzzleActive =
      (this.gameState.wram.wUnownState ?? 0) !== 0 ||
      getUnownOverlayLockDepth(this.gameState) > 0;
    if (this.fatalError) {
      this.ui.clearScreen([0, 0, 0]);
      const message = String(this.fatalError?.message ?? this.fatalError);
      const text = [
        "FATAL ERROR",
        "",
        message,
        "",
        "Check terminal logs and .next/dev/logs/next-development.log",
      ].join("\n");
      const font = this.ui.font;
      const render = font?.render_text ?? font?.renderText ?? null;
      if (typeof render === "function") {
        try {
          render.call(font, text, TILE_SIZE, TILE_SIZE, this.ui.screen, { uppercase: false });
        } catch {
          // ignore (fallback to blank screen)
        }
      }
      this.ui.update();
      return;
    }
    if (isUnownPuzzleActive) {
      return;
    }
    const uiWithSnapshot = this.ui as unknown as {
      getSnapshot?: () => { viewportTitle?: string | null } | null;
    };
    const activeTextSnapshot =
      typeof uiWithSnapshot.getSnapshot === "function" ? uiWithSnapshot.getSnapshot() : null;
    const shouldPreserveInputCaptureOverlay =
      this.currentState === "overworld" &&
      Boolean(this.overworld?.input_capture_active) &&
      Boolean(activeTextSnapshot?.viewportTitle) &&
      activeTextSnapshot?.viewportTitle !== "Overworld";
    if (shouldPreserveInputCaptureOverlay) {
      return;
    }
    // A tileset refresh intentionally releases the old map surfaces while its
    // replacement loads. Do not present the clear that normally starts a
    // frame in that interval: it would replace the last complete overworld
    // frame with black until the asynchronous rebuild finishes.
    if (this.currentState === "overworld" && !this.hasCompleteOverworldFrame()) {
      return;
    }
    this.ui.clearScreen([0, 0, 0]);
    const shouldDrawBootPixels = !(this.ui instanceof TextUI);
    switch (this.currentState) {
      case "intro":
        if (shouldDrawBootPixels) {
          this.introSequence?.draw(this.getBootRenderContext());
        }
        if (this.introSequence) {
          renderTextSnapshot(this.ui, this.introSequence.getTextSnapshot());
        }
        break;
      case "title":
        if (shouldDrawBootPixels) {
          this.titleScreen?.draw(this.getBootRenderContext());
        }
        if (this.titleScreen) {
          renderTextSnapshot(this.ui, this.titleScreen.getTextSnapshot());
        }
        break;
      case "main_menu":
        this.mainMenu?.draw();
        break;
      case "continue":
        this.continueScreen?.draw();
        break;
      case "delete_save":
        this.deleteSaveScreen?.draw();
        break;
      case "clock_reset":
        this.clockResetScreen?.draw();
        break;
      case "gender":
        if (shouldDrawBootPixels) {
          this.genderSelection?.draw(this.getBootRenderContext());
        }
        if (this.genderSelection) {
          renderTextSnapshot(this.ui, this.genderSelection.getTextSnapshot());
        }
        break;
      case "oak_intro":
        if (shouldDrawBootPixels) {
          this.oakIntroSequence?.draw(this.getBootRenderContext());
        }
        if (this.oakIntroSequence) {
          renderTextSnapshot(this.ui, this.oakIntroSequence.getTextSnapshot());
        }
        break;
      case "name_entry":
        this.playerNameEntryScreen?.draw();
        break;
      case "overworld":
        this.overworld.draw();
        this.flushPendingOverworldMusicAction();
        break;
      case "battle":
        if (this.battleUi) {
          this.battleUi.presented_this_frame = false;
        }
        if (this.battle) {
          this.battle.draw();
        }
        break;
      case "menu":
        if (this.overworld) {
          const overworldWithSnapshotSuppression = this.overworld as Overworld & {
            _suppress_text_snapshot?: boolean;
          };
          overworldWithSnapshotSuppression._suppress_text_snapshot = true;
          try {
            this.overworld.draw();
          } finally {
            overworldWithSnapshotSuppression._suppress_text_snapshot = false;
          }
        }
        this.menuState.draw();
        break;
      // Other states...
    }
    if (!(this.currentState === "battle" && this.battleUi?.presented_this_frame)) {
      this.ui.update();
    }
  }

  private hasCompleteOverworldFrame(): boolean {
    const overworld = this.overworld as Overworld & {
      map_surface?: Surface | null;
      _composite_surface?: Surface | null;
    };
    return Boolean(overworld._composite_surface ?? overworld.map_surface);
  }

  private handleInput(): void {
    if (this.eventQueue.length === 0) {
      return;
    }
    const isUnownPuzzleActive =
      (this.gameState.wram.wUnownState ?? 0) !== 0 ||
      getUnownOverlayLockDepth(this.gameState) > 0;
    if (this.currentState === "overworld" && this.overworld?.input_capture_active) {
      return;
    }
    // Unown puzzle overlays consume input from the shared queue while the runner is paused.
    if (this.currentState === "overworld" && isUnownPuzzleActive) {
      return;
    }
    if (isUnownPuzzleActive) {
      return;
    }
    const events = gameEngine.event.get(this.eventQueue);
    this._latchJoypadForFrame(events);
    for (const event of events) {
      if (event.type === gameEngine.QUIT) {
        this.quitRequested = true;
        continue;
      }
      if (this.currentState === "intro") {
        if (this.introSequence?.handleInput(event)) {
          this.startBootTransition(() => {
            this.enterTitleState();
          });
        }
        continue;
      }
      if (this.currentState === "title") {
        if (this.titleScreen) {
          this.titleScreen.handleInput(
            {
              key: this.getBootScreenInputKey(event),
            } as KeyboardEvent,
            isKeyDownEvent(event)
          );
        }
        continue;
      }
      if (this.currentState === "main_menu") {
        if (!this.mainMenu) {
          continue;
        }
        const action = this.mainMenu.handleInput(event);
        if (action === "new_game") {
          this.startBootTransition(async () => {
            await this.enterNewGameFlow();
          });
          continue;
        }
        if (action === "show_continue_screen") {
          this.startBootTransition(() => {
            this.enterContinueState();
          });
          continue;
        }
        if (action === "options_menu") {
          this.menuState.openOptionsMenu();
          this.currentState = "menu";
          continue;
        }
        continue;
      }
      if (this.currentState === "continue") {
        const action = this.continueScreen?.handleInput(event);
        if (action === "confirm") {
          this.startBootTransition(async () => {
            try {
              const activeBootSaveSlot =
                (await this.resolveActiveBootSaveSlot()) ?? this.bootSaveSlot;
              const loadedState = await loadGame(activeBootSaveSlot);
              this.hasPersistedSaveData = true;
              await this.transitionToOverworldState(loadedState);
            } catch (error) {
              logger.error("[game] Continue load failed", error);
              this.enterTitleState();
            }
          });
        } else if (action === "cancel") {
          this.startBootTransition(() => {
            this.enterTitleState();
          });
        }
        continue;
      }
      if (this.currentState === "delete_save") {
        const action = this.deleteSaveScreen?.handleInput(event);
        if (action === "confirm") {
          this.startBootTransition(async () => {
            try {
              const deletedSlots = await Promise.all(
                resolveBootSaveCandidates(this.bootSaveSlot).map((slot) => deleteSaveGame(slot))
              );
              this.hasPersistedSaveData = !deletedSlots.some(Boolean)
                ? (await this.resolveActiveBootSaveSlot()) !== null
                : false;
            } catch (error) {
              logger.error("[game] Delete save failed", error);
            }
            this.enterTitleState();
          });
        } else if (action === "cancel") {
          this.startBootTransition(() => {
            this.enterTitleState();
          });
        }
        continue;
      }
      if (this.currentState === "clock_reset") {
        const action = this.clockResetScreen?.handleInput(event);
        if (action === "confirm" || action === "cancel") {
          this.startBootTransition(() => {
            this.enterTitleState();
          });
        }
        continue;
      }
      if (this.currentState === "gender") {
        this.genderSelection?.handleInput(event);
        continue;
      }
      if (this.currentState === "name_entry") {
        this.playerNameEntryScreen?.handleInput(event);
        continue;
      }
      if (this.currentState === "oak_intro") {
        this.oakIntroSequence?.handleInput(event as KeyboardEvent);
        continue;
      }
      if (this.currentState === "menu") {
        const action = this.menuState.handleInput(event);
        if (action === "save") {
          logger.info("[game] Save action selected from menu");
          this.menuState.beginSaveFlow({
            saveExists: Boolean(this.gameState.sram.player_name?.trim()),
            saveCallback: async () => {
              logger.info("[game] Save callback dispatching saveGame");
              return saveGameWithHistory(
                this.gameState,
                MANUAL_SAVE_SLOT,
                MANUAL_SAVE_HISTORY_SLOTS
              );
            },
          });
          continue;
        }
        if (action === "overworld" || action === "close_menu") {
          this.currentState = "overworld";
        }
        continue;
      }
      if (this.currentState === "battle") {
        this.battle?.handle_input?.(event);
        continue;
      }
      if (this.currentState === "overworld") {
        if (isStartEvent(event) && this._canOpenStartMenu()) {
          this.menuState.reset();
          this.currentState = "menu";
          continue;
        }
        this.overworld.handle_input(event);
      }
    }
  }

  private _latchJoypadForFrame(events: ReadonlyArray<InstanceType<typeof gameEngine.event.Event>>): void {
    // ASM mapping: home/joypad.asm::GetJoypad computes a single per-frame delta
    // from the final latched button state rather than per-event edge updates.
    const joypad = this.gameState.hram.joypad;
    const previous = joypad.hJoyDown & 0xff;
    let current = previous;
    for (const event of events) {
      const bit = joypad_bits_for_event(event);
      if (bit === undefined) {
        continue;
      }
      if (isKeyDownEvent(event)) {
        current |= bit;
        continue;
      }
      if (isKeyUpEvent(event)) {
        current &= ~bit;
      }
    }
    current &= 0xff;
    const delta = previous ^ current;
    const pressed = delta & current;
    const released = delta & previous;

    joypad.hJoyPressed = pressed;
    joypad.hJoyReleased = released;
    joypad.hJoyDown = current;
    joypad.hJoypadPressed = pressed;
    joypad.hJoypadReleased = released;
    joypad.hJoypadDown = current;
    joypad.hJoyLast = current;
    joypad.hJoypadSum = current;
  }

  private _canOpenStartMenu(): boolean {
    const overworld = this.overworld ?? null;
    if (!overworld) {
      return false;
    }
    if (overworld.is_moving) {
      return false;
    }
    if (typeof overworld.player_movement_locked === "function" && overworld.player_movement_locked()) {
      return false;
    }
    if (typeof overworld.script_tasks_active === "function" && overworld.script_tasks_active()) {
      return false;
    }
    const runner = overworld.script_runner ?? null;
    if (runner && (runner.state !== "idle" && runner.state !== 0)) {
      return false;
    }
    const dialogue = overworld.dialogue ?? null;
    if (dialogue && (dialogue.active || dialogue.waiting_for_input || dialogue.is_script_paused)) {
      return false;
    }
    return true;
  }

  private gameLoop(timestamp?: number) {
    const now = this._resolveTimestampMs(timestamp);
    if (this.lastFrameTimeMs === null) {
      this.lastFrameTimeMs = now;
      this.frameRemainderMs = FRAME_DURATION_MS;
    } else {
      const delta = Math.max(0, now - this.lastFrameTimeMs);
      this.lastFrameTimeMs = now;
      this.frameRemainderMs = Math.min(this.frameRemainderMs + delta, MAX_ACCUMULATED_MS);
    }

    let framesToProcess = Math.floor(this.frameRemainderMs / FRAME_DURATION_MS);
    if (framesToProcess > 0) {
      framesToProcess = Math.min(framesToProcess, MAX_TICKS_PER_FRAME);
      this.frameRemainderMs -= framesToProcess * FRAME_DURATION_MS;
      for (let i = 0; i < framesToProcess; i += 1) {
        try {
          this.tick();
          this.loopCircuitBreaker();
        } catch (error) {
          this._failFatal(error);
          break;
        }
      }
    }
    if (!this.quitRequested && !this.fatalError) {
      this.loopCircuitBreaker = createCircuitBreaker(10000, "game.gameLoop");
      this._scheduleGameLoop(this._nextLoopDelayMs());
    } else {
      this._clearScheduledLoop();
    }
  }

  private _resetFrameTiming(): void {
    this._clearScheduledLoop();
    this.lastFrameTimeMs = null;
    this.frameRemainderMs = 0;
  }

  private _nextLoopDelayMs(): number {
    const remaining = FRAME_DURATION_MS - this.frameRemainderMs;
    if (!Number.isFinite(remaining)) {
      return FRAME_DURATION_MS;
    }
    return Math.max(0, Math.min(FRAME_DURATION_MS, remaining));
  }

  private _scheduleGameLoop(delayMs: number): void {
    this._clearScheduledLoop();
    this.loopTimerId = setTimeout(() => {
      this.loopTimerId = null;
      this.gameLoopBound();
    }, Math.max(0, delayMs));
  }

  private _clearScheduledLoop(): void {
    if (this.loopTimerId !== null) {
      clearTimeout(this.loopTimerId);
      this.loopTimerId = null;
    }
  }

  private _resolveTimestampMs(timestamp?: number): number {
    if (typeof timestamp === "number" && Number.isFinite(timestamp)) {
      return timestamp;
    }
    if (typeof performance !== "undefined" && typeof performance.now === "function") {
      return performance.now();
    }
    return Date.now();
  }

  private _getVramManager(): VRAMManager {
    if (!this.vramManager) {
      this.vramManager = new VRAMManager(this.gameState.vram);
    }
    return this.vramManager;
  }

  private _runAutosave(): Promise<void> {
    return saveGame(this.gameState, this.autosaveSlot)
      .then((saved) => {
        if (saved) {
          this.hasPersistedSaveData = true;
        }
      })
      .catch((error) => {
        console.warn("[save] Autosave failed:", error);
      });
  }

  private async handleBattleCompleteAutosave(event?: Event): Promise<void> {
    const battleResult = Number((event?.data as { result?: unknown } | undefined)?.result ?? this.gameState.wram.battle_result ?? 0);
    const party = this.gameState.sram.party?.pokemon ?? [];
    const hasUsablePartyMember = party.some((pokemon) =>
      Boolean(
        pokemon &&
        (pokemon.hp ?? 0) > 0 &&
        pokemon.species?.id?.toUpperCase() !== "EGG" &&
        (pokemon.nickname ?? "").toUpperCase() !== "EGG"
      )
    );
    if (battleResult === 1 && !hasUsablePartyMember) {
      return;
    }
    this.autosaveStepCounter = 0;
    await this._runAutosave();
  }

  private async handlePlayerStepAutosave(event: Event): Promise<void> {
    const rawCount = (event.data as { count?: number } | undefined)?.count;
    const count = Number.isFinite(rawCount) ? Math.max(0, Number(rawCount)) : 1;
    if (count <= 0) {
      return;
    }
    this.autosaveStepCounter += count;
    if (this.autosaveStepCounter < AUTOSAVE_STEP_THRESHOLD) {
      return;
    }
    this.autosaveStepCounter = 0;
    await this._runAutosave();
  }

  private _installFatalHandlers(): void {
    if (typeof window === "undefined") {
      return;
    }
    window.addEventListener("error", (event) => {
      this._failFatal(event.error ?? event.message ?? event);
    });
    window.addEventListener("unhandledrejection", (event) => {
      this._failFatal((event as PromiseRejectionEvent).reason ?? event);
    });
  }

  private _failFatal(error: unknown): void {
    if (this.fatalError) {
      return;
    }
    const asError = error instanceof Error ? error : new Error(String(error));
    this.fatalError = asError;
    this.quitRequested = true;
    console.error("[fatal] Game loop crashed:", asError);
  }
}

const hasRenderableFont = (
  font?: BaseFontRenderer | null,
): font is BaseFontRenderer & { renderText: NonNullable<MenuFontRenderer["renderText"]> } => {
  if (!font) {
    return false;
  }
  const renderFn = font.renderText ?? font.render_text;
  return typeof renderFn === "function";
};

const hasFontTiles = (
  font?: BaseFontRenderer | null,
): font is BaseFontRenderer & { font_tiles: Record<number, Surface> } => {
  return Boolean(font && (font.font_tiles || font.fontTiles));
};

const assertMenuUI: (ui: BaseUI) => asserts ui is BaseUI & MenuUI = (ui) => {
  if (!hasRenderableFont(ui.font ?? null)) {
    throw new Error("Game UI requires a font renderer with renderText.");
  }
};

const assertBattleUI: (ui: BaseUI) => asserts ui is BaseUI & BattleUI = (ui) => {
  assertMenuUI(ui);
  const font = ui.font;
  if (font && !font.font_tiles && font.fontTiles) {
    font.font_tiles = font.fontTiles;
  }
  if (!hasFontTiles(font ?? null)) {
    throw new Error("Battle UI requires font tiles.");
  }
};
