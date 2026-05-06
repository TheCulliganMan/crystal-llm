import { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import { GameState } from "@pokecrystal/core/core/state";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { DexJumptableState, PokedexScreen as MenuPokedexScreen } from "@pokecrystal/core/ui/menus/pokedex";
import { resetPokedexHardwareState } from "@pokecrystal/core/ui/menus/pokedex-assets";
import type { KeyEvent } from "@pokecrystal/core/input/buttons";
import type { FontRenderer, MenuUI } from "@pokecrystal/core/ui/menus/types";
import type { GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import type { RenderTextOptions, SurfaceLike } from "@pokecrystal/core/ui/font-renderer";
import { Surface } from "@pokecrystal/core/ui/surface";

type MenuEvent = Parameters<MenuPokedexScreen["handleInput"]>[0];
type MenuOptions = NonNullable<ConstructorParameters<typeof MenuPokedexScreen>[2]>;
type MenuLike = Pick<
  MenuPokedexScreen,
  "reset" | "draw" | "handleInput" | "setJumptableState" | "redisplayEntryScreen"
> & { cursorOamEntries?: unknown };

const isMenuInput = (value: unknown): value is MenuEvent => {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const maybeEvent = value as { is_press?: unknown; direction?: unknown; button?: unknown };
  return (
    "is_press" in maybeEvent ||
    "direction" in maybeEvent ||
    "button" in maybeEvent ||
    "code" in maybeEvent
  );
};

export { DexJumptableState };

export class PokedexScreen {
  private readonly menu: MenuLike;
  private queuedEvent: MenuEvent | KeyboardEvent | null = null;

  constructor(
    private readonly ui: ScreenUI,
    private readonly gameState: GameState,
    private readonly audioEngineOrOptions: AudioEngine | MenuOptions | null = null,
  ) {
    resetPokedexHardwareState();
    const menuUI = this.toMenuUI(ui);
    const options = this.resolveMenuOptions(audioEngineOrOptions);
    this.menu = new MenuPokedexScreen(menuUI, gameState, options);
  }

  get blackoutActive(): boolean {
    return false;
  }

  get cursorOamEntries(): unknown {
    return this.menu.cursorOamEntries;
  }

  queueEvent(event: MenuEvent | KeyboardEvent): void {
    this.queuedEvent = event;
  }

  handleInput(event: MenuEvent | KeyboardEvent | null): string | null {
    return this.step(event);
  }

  step(event: MenuEvent | KeyboardEvent | null = null): string | null {
    const activeEvent = event ?? this.queuedEvent;
    this.queuedEvent = null;
    const state = this.gameState.wram.wJumptableIndex as unknown as DexJumptableState;

    switch (state) {
      case DexJumptableState.MAIN_SCR:
        return this.mainScreen(activeEvent);
      case DexJumptableState.UPDATE_MAIN_SCR:
        return this.updateMainScreen(activeEvent);
      case DexJumptableState.DEX_ENTRY_SCR:
        return this.dexEntryScreen(activeEvent);
      case DexJumptableState.UPDATE_DEX_ENTRY_SCR:
        return this.updateEntryScreen(activeEvent);
      case DexJumptableState.REINIT_DEX_ENTRY_SCR:
        return this.reinitEntryScreen(activeEvent);
      case DexJumptableState.SEARCH_SCR:
        return this.searchScreen(activeEvent);
      case DexJumptableState.UPDATE_SEARCH_SCR:
        return this.updateSearchScreen(activeEvent);
      case DexJumptableState.OPTION_SCR:
        return this.optionScreen(activeEvent);
      case DexJumptableState.UPDATE_OPTION_SCR:
        return this.updateOptionScreen(activeEvent);
      case DexJumptableState.SEARCH_RESULTS_SCR:
        return this.searchResultsScreen(activeEvent);
      case DexJumptableState.UPDATE_SEARCH_RESULTS_SCR:
        return this.updateSearchResultsScreen(activeEvent);
      case DexJumptableState.UNOWN_MODE:
        return this.unownModeScreen(activeEvent);
      case DexJumptableState.UPDATE_UNOWN_MODE:
        return this.updateUnownModeScreen(activeEvent);
      case DexJumptableState.EXIT:
        return this.exit();
      default:
        throw new Error(`Unhandled Pokédex jumptable state ${this.gameState.wram.wJumptableIndex}`);
    }
  }

  private mainScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    resetPokedexHardwareState();
    this.menu.reset();
    this.menu.setJumptableState(DexJumptableState.UPDATE_MAIN_SCR);
    return this.updateMainScreen(event);
  }

  private updateMainScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    const action = this.dispatchEvent(event);
    this.drawWithBlackout();
    return action;
  }

  private dexEntryScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    this.menu.setJumptableState(DexJumptableState.UPDATE_DEX_ENTRY_SCR);
    return this.updateEntryScreen(event);
  }

  private updateEntryScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    const action = this.dispatchEvent(event);
    this.drawWithBlackout();
    return action;
  }

  private reinitEntryScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    this.menu.redisplayEntryScreen();
    this.menu.setJumptableState(DexJumptableState.UPDATE_DEX_ENTRY_SCR);
    return this.updateEntryScreen(event);
  }

  private searchScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    this.menu.setJumptableState(DexJumptableState.UPDATE_SEARCH_SCR);
    return this.updateSearchScreen(event);
  }

  private updateSearchScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    const action = this.dispatchEvent(event);
    this.drawWithBlackout();
    return action;
  }

  private optionScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    this.menu.setJumptableState(DexJumptableState.UPDATE_OPTION_SCR);
    return this.updateOptionScreen(event);
  }

  private updateOptionScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    const action = this.dispatchEvent(event);
    this.drawWithBlackout();
    return action;
  }

  private searchResultsScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    this.menu.setJumptableState(DexJumptableState.UPDATE_SEARCH_RESULTS_SCR);
    return this.updateSearchResultsScreen(event);
  }

  private updateSearchResultsScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    const action = this.dispatchEvent(event);
    this.drawWithBlackout();
    return action;
  }

  private unownModeScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    this.menu.setJumptableState(DexJumptableState.UPDATE_UNOWN_MODE);
    return this.updateUnownModeScreen(event);
  }

  private updateUnownModeScreen(event: MenuEvent | KeyboardEvent | null): string | null {
    const action = this.dispatchEvent(event);
    this.drawWithBlackout();
    return action;
  }

  private exit(): string {
    this.gameState.hram.hInMenu = 0;
    return "exit";
  }

  private dispatchEvent(event: MenuEvent | KeyboardEvent | null): string | null {
    if (!event) {
      return null;
    }
    return this.menu.handleInput(this.normalizeEvent(event));
  }

  private drawWithBlackout(): void {
    this.menu.draw();
  }

  private toMenuUI(ui: ScreenUI): MenuLikeUI {
    const legacyContextPalette = (
      ui as { get_context_palette?: (name: string) => [number, number, number][] }
    ).get_context_palette;
    const modernContextPalette = (
      ui as { getContextPalette?: (name: string) => [number, number, number][] }
    ).getContextPalette;
    const getPokemonFrontSurface = (
      ui as { getPokemonFrontSurface?: (speciesId: string, frame?: number) => Surface | null }
    ).getPokemonFrontSurface;
    const update = (ui as { update?: () => void }).update;
    return {
      screen: ui.screen,
      tileSize: this.resolveTileSize(ui),
      font: this.buildFontRenderer(ui.font),
      getPokemonFrontSurface: getPokemonFrontSurface?.bind(ui),
      eventQueue: (ui as { eventQueue?: GameEngineEventQueue }).eventQueue,
      drawWindow: this.resolveDrawWindow(ui),
      get_context_palette: legacyContextPalette ? legacyContextPalette.bind(ui) : undefined,
      getContextPalette: modernContextPalette ? modernContextPalette.bind(ui) : undefined,
      update: update?.bind(ui),
    };
  }

  private resolveMenuOptions(audioEngineOrOptions: AudioEngine | MenuOptions | null): MenuOptions {
    if (this.isAudioEngine(audioEngineOrOptions)) {
      return { audioEngine: audioEngineOrOptions };
    }
    if (this.isMenuScreenOptions(audioEngineOrOptions)) {
      return audioEngineOrOptions;
    }
    return {};
  }

  private isAudioEngine(value: AudioEngine | MenuOptions | null): value is AudioEngine {
    if (!value || typeof value !== "object") {
      return false;
    }
    return (
      "sounds" in value &&
      "music" in value &&
      "masterVolume" in value &&
      "muted" in value &&
      typeof value === "object"
    );
  }

  private isMenuScreenOptions(
    value: AudioEngine | MenuOptions | null
  ): value is MenuOptions {
    if (!value || typeof value !== "object") {
      return false;
    }
    if (this.isAudioEngine(value)) {
      return false;
    }
    return (
      "audioEngine" in value ||
      "dataLoader" in value ||
      "scriptRunner" in value ||
      "printer" in value
    );
  }

  private resolveTileSize(ui: ScreenUI): number {
    return (ui as { tileSize?: number }).tileSize ??
      (ui as { tile_size?: number }).tile_size ??
      8;
  }

  private resolveDrawWindow(ui: ScreenUI): MenuUI["drawWindow"] {
    const drawWindow = (ui as { drawWindow?: MenuUI["drawWindow"] }).drawWindow;
    if (drawWindow) {
      return (surface, x, y, width, height, options) => {
        drawWindow.call(ui, surface, x, y, width, height, options);
      };
    }
    const drawWindowLegacy = (
      ui as {
        draw_window?: (
          surface: Surface,
          x: number,
          y: number,
          width: number,
          height: number,
          options?: {
            frame_id?: number | null;
            fill?: [number, number, number] | null;
            z_index?: number;
            record?: boolean;
          },
        ) => void;
      }
    ).draw_window;
    if (!drawWindowLegacy) {
      throw new Error("PokédexScreen requires ui.drawWindow or ui.draw_window.");
    }
    return (surface, x, y, width, height, options) => {
      const opts = options as {
        frameId?: number | null;
        fill?: [number, number, number] | null;
        zIndex?: number;
        record?: boolean;
      };
      drawWindowLegacy.call(ui, surface, x, y, width, height, {
        frame_id: opts.frameId,
        fill: opts.fill,
        z_index: opts.zIndex,
        record: opts.record,
      });
    };
  }

  private buildFontRenderer(font: ScreenUI["font"]): FontRenderer {
    const renderer = font as {
      renderText?: (
        text: string,
        x: number,
        y: number,
        surface: Surface | SurfaceLike,
        options?: RenderTextOptions | boolean
      ) => void;
      render_text?: (
        text: string,
        x: number,
        y: number,
        surface: Surface | SurfaceLike,
        options?: RenderTextOptions | boolean
      ) => void;
      paletteVariants?: FontRenderer["paletteVariants"];
      fontTiles?: Record<number, Surface>;
      font_tiles?: Record<number, Surface>;
    };
    const renderText = renderer.renderText ?? renderer.render_text;
    if (!renderText) {
      throw new Error("PokédexScreen requires a font renderer");
    }
    return {
      renderText: (text, x, y, targetSurface, options) => {
        renderText.call(font, text, x, y, targetSurface as never, options as never);
      },
      paletteVariants: renderer.paletteVariants,
      fontTiles: renderer.fontTiles,
      font_tiles: renderer.font_tiles,
    };
  }

  private normalizeEvent(event: MenuEvent | KeyboardEvent): MenuEvent {
    if (isMenuInput(event)) {
      return event;
    }
    if ("type" in event && "key" in event) {
      return {
        ...(event as KeyboardEvent),
        type: event.type,
        key: event.key,
        code: event.code,
        is_press: event.type === "keydown",
      } as KeyEvent;
    }
    return event as MenuEvent;
  }
}

type MenuLikeUI = {
  screen: ScreenUI["screen"];
  tileSize: number;
  font: FontRenderer;
  getPokemonFrontSurface?: (speciesId: string, frame?: number) => Surface | null;
  drawWindow: MenuUI["drawWindow"];
  eventQueue?: GameEngineEventQueue;
  get_context_palette?: (name: string) => [number, number, number][];
  getContextPalette?: (name: string) => [number, number, number][];
  update?: () => void;
};
