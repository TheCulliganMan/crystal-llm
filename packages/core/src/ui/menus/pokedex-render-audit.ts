// ASM audit helper: render every Pokédex entry page plus key menu states.
import { NUM_POKEMON, NUM_UNOWN } from "../../core/constants";
import { loadAllSpecies } from "../../core/data-loader";
import { DexMode } from "../../core/enums/pokedex";
import { PokemonType } from "../../core/enums/pokemon";
import type { PokemonSpecies } from "../../core/models";
import { createInitialGameState } from "../../core/state";
import { setPokedexFlag } from "../../core/pokedex";
import { BaseUI } from "../base-ui";
import type { BaseFontRenderer } from "../base-ui";
import { Surface } from "../surface";
import { TextRenderer } from "../text/text-renderer";
import { drawPokedexList, drawSearchResultsWindow, drawSearchScreen, LIST_WINDOW_LENGTH } from "./pokedex-layout";
import { drawEntryPage, drawMainSidebar, drawOptionScreen, drawSearchResultsBackground, drawUnownModeScreen } from "./pokedex-render";
import { parsePokedexEntryFile } from "./pokedex-entry-loader";
import type { DexEntry } from "./pokedex-state";
import { orderEntriesForMode } from "./pokedex-state";

const SCREEN_WIDTH = 160;
const SCREEN_HEIGHT = 144;
const POKEDEX_SCX = 5;
const SEARCH_RESULTS_WINDOW_X = 0x4a;
const ENTRY_ACTIONS = ["PAGE", "AREA", "CRY", "PRNT"];
const ENTRY_ACTION_COORDINATES: Array<[number, number]> = [
  [1, 2],
  [6, 7],
  [11, 12],
  [15, 16],
];

type AuditFrameKind = "entry" | "screen";
type AuditFontOptions = {
  color?: [number, number, number];
  textWidth?: number;
  maxLines?: number;
  uppercase?: boolean;
};
type AuditFont = BaseFontRenderer & {
  renderText: (
    text: string,
    x: number,
    y: number,
    surface: Surface,
    options?: AuditFontOptions,
  ) => void;
  render_text: (
    text: string,
    x: number,
    y: number,
    surface: Surface,
    options?: AuditFontOptions,
  ) => void;
  fontTiles: Record<number, Surface>;
  font_tiles: Record<number, Surface>;
  paletteVariants: NonNullable<BaseFontRenderer["paletteVariants"]>;
  getCharTile: NonNullable<BaseFontRenderer["getCharTile"]>;
};

export type PokedexAuditFrame = {
  kind: AuditFrameKind;
  slug: string;
  surface: Surface;
  speciesId?: string;
  pokedexNumber?: number;
  pageIndex?: number;
  pageCount?: number;
};

export type PokedexAuditSummary = {
  entrySpeciesCount: number;
  entryPageCount: number;
  screenCount: number;
  totalFrameCount: number;
};

export const createPokedexAuditRunId = (): string =>
  new Date().toISOString().replace(/[:.]/g, "-");

export const createPokedexAuditFilename = (slug: string, runId: string = createPokedexAuditRunId()): string =>
  `${slug}-${runId}.png`;

type CachedDexEntry = DexEntry & {
  entryData: ReturnType<typeof parsePokedexEntryFile>;
};

class HeadlessPokedexAuditUI extends BaseUI {
  public readonly font: AuditFont;
  private readonly textRenderer: TextRenderer;

  private constructor(font: TextRenderer) {
    super(SCREEN_WIDTH, SCREEN_HEIGHT, 1);
    this.textRenderer = font;
    this.font = {
      renderText: (text, x, y, surface, options) => {
        this.textRenderer.renderText(text, x, y, surface, options);
      },
      render_text: (text, x, y, surface, options) => {
        this.textRenderer.render_text(
          text,
          x,
          y,
          surface,
          options && typeof options !== "boolean" ? options : undefined,
        );
      },
      fontTiles: this.textRenderer.fontTiles,
      font_tiles: this.textRenderer.font_tiles,
      paletteVariants: this.textRenderer.paletteVariants.bind(this.textRenderer),
      getCharTile: this.textRenderer.getCharTile.bind(this.textRenderer),
    };
  }

  static async create(): Promise<HeadlessPokedexAuditUI> {
    const font = new TextRenderer();
    await font.load();
    return new HeadlessPokedexAuditUI(font);
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  update(): void {
    return;
  }

  override getPokemonFrontSurface(speciesId: string, frame: number = 0): Surface | null {
    const normalized = speciesId.toUpperCase() === "UNOWN" ? "unown_a" : speciesId;
    return super.getPokemonFrontSurface(normalized, frame);
  }
}

const buildSpeciesEntries = (): DexEntry[] => {
  const byId = Array.from(loadAllSpecies().values()).sort((left, right) => left.int_id - right.int_id);
  return byId.map((species) => ({
    pokedexNumber: species.int_id,
    species,
  }));
};

let cachedRenderableEntries: CachedDexEntry[] | null = null;

const getRenderableEntries = (): CachedDexEntry[] => {
  if (cachedRenderableEntries) {
    return cachedRenderableEntries;
  }

  cachedRenderableEntries = buildSpeciesEntries().map((entry) => {
    const entryData = parsePokedexEntryFile(entry.species.id);
    if (!entryData.pages.length) {
      throw new Error(`Pokédex entry '${entry.species.id}' has no renderable pages.`);
    }
    return {
      ...entry,
      entryData,
    };
  });

  return cachedRenderableEntries;
};

const buildFullDexState = () => {
  const gameState = createInitialGameState();
  const speciesEntries = getRenderableEntries();
  for (const entry of speciesEntries) {
    setPokedexFlag(gameState, entry.species.int_id, "seen");
    setPokedexFlag(gameState, entry.species.int_id, "owned");
  }
  return { gameState, speciesEntries };
};

const slugForEntryPage = (entry: DexEntry, pageIndex: number): string => {
  return `${String(entry.pokedexNumber).padStart(3, "0")}-${entry.species.id.toLowerCase()}-page-${pageIndex + 1}`;
};

const createBlankScreen = (): Surface => new Surface(SCREEN_WIDTH, SCREEN_HEIGHT);

const pokedexWindowXForMode = (mode: DexMode): number => {
  return mode === DexMode.OLD ? 0x4a : 0x47;
};

const blitLayer = (
  dest: Surface,
  source: Surface,
  destX: number,
  destY: number,
  area?: { x: number; y: number; width: number; height: number },
): void => {
  dest.blit(source, [destX, destY], area);
};

const blitScrolledLayer = (dest: Surface, source: Surface, scrollX: number, scrollY: number): void => {
  const [sourceWidth, sourceHeight] = source.get_size();
  const [destWidth, destHeight] = dest.get_size();
  if (!sourceWidth || !sourceHeight) {
    return;
  }
  const withinX = scrollX >= 0 && scrollX + destWidth <= sourceWidth;
  const withinY = scrollY >= 0 && scrollY + destHeight <= sourceHeight;
  if (withinX && withinY) {
    blitLayer(dest, source, 0, 0, { x: scrollX, y: scrollY, width: destWidth, height: destHeight });
    return;
  }

  const normX = ((scrollX % sourceWidth) + sourceWidth) % sourceWidth;
  const normY = ((scrollY % sourceHeight) + sourceHeight) % sourceHeight;
  const primaryWidth = Math.min(sourceWidth - normX, destWidth);
  const primaryHeight = Math.min(sourceHeight - normY, destHeight);
  blitLayer(dest, source, 0, 0, { x: normX, y: normY, width: primaryWidth, height: primaryHeight });
  const remainingWidth = destWidth - primaryWidth;
  const remainingHeight = destHeight - primaryHeight;
  if (remainingWidth > 0) {
    blitLayer(dest, source, primaryWidth, 0, { x: 0, y: normY, width: remainingWidth, height: primaryHeight });
  }
  if (remainingHeight > 0) {
    blitLayer(dest, source, 0, primaryHeight, { x: normX, y: 0, width: primaryWidth, height: remainingHeight });
  }
  if (remainingWidth > 0 && remainingHeight > 0) {
    blitLayer(dest, source, primaryWidth, primaryHeight, { x: 0, y: 0, width: remainingWidth, height: remainingHeight });
  }
};

const renderLayeredMainScreen = (
  ui: HeadlessPokedexAuditUI,
  entries: DexEntry[],
  seenSet: Set<number>,
  caughtSet: Set<number>,
  mode: DexMode,
): Surface => {
  const bgWidth = (20 + Math.ceil(POKEDEX_SCX / 8)) * 8;
  const bgLayer = new Surface(bgWidth, SCREEN_HEIGHT);
  const windowLayer = createBlankScreen();
  const finalScreen = createBlankScreen();

  drawMainSidebar(ui, bgLayer, {
    seenCount: seenSet.size,
    caughtCount: caughtSet.size,
    activeSpeciesId: entries[0]?.species.id ?? null,
  });
  drawPokedexList(
    ui,
    windowLayer,
    entries,
    0,
    0,
    seenSet,
    caughtSet,
    mode,
    LIST_WINDOW_LENGTH,
    {
      windowPrompts: true,
      originOffset: [0, 0],
    },
  );

  blitScrolledLayer(finalScreen, bgLayer, POKEDEX_SCX, 0);
  const windowLeft = pokedexWindowXForMode(mode) - 7;
  blitLayer(finalScreen, windowLayer, windowLeft, 0);
  return finalScreen;
};

const renderLayeredSearchResultsScreen = (
  ui: HeadlessPokedexAuditUI,
  entries: DexEntry[],
  seenSet: Set<number>,
  caughtSet: Set<number>,
  typeIndexes: [number, number],
): Surface => {
  const bgWidth = (20 + Math.ceil(POKEDEX_SCX / 8)) * 8;
  const bgLayer = new Surface(bgWidth, SCREEN_HEIGHT);
  const windowLayer = createBlankScreen();
  const finalScreen = createBlankScreen();

  drawSearchResultsBackground(ui, bgLayer, {
    resultCount: entries.length,
    activeSpeciesId: entries[0]?.species.id ?? null,
  });
  drawSearchResultsWindow(
    ui,
    windowLayer,
    entries,
    0,
    0,
    typeIndexes,
    seenSet,
    caughtSet,
    DexMode.NEW,
    4,
  );

  blitScrolledLayer(finalScreen, bgLayer, POKEDEX_SCX, 0);
  blitLayer(finalScreen, windowLayer, SEARCH_RESULTS_WINDOW_X - 7, 0);
  return finalScreen;
};

const renderEntryFrames = async (ui: HeadlessPokedexAuditUI): Promise<PokedexAuditFrame[]> => {
  const frames: PokedexAuditFrame[] = [];
  const entries = getRenderableEntries();
  if (entries.length !== NUM_POKEMON) {
    throw new Error(`Expected ${NUM_POKEMON} Pokédex entries, found ${entries.length}.`);
  }

  for (const entry of entries) {
    const { entryData } = entry;
    for (let pageIndex = 0; pageIndex < entryData.pages.length; pageIndex += 1) {
      const surface = createBlankScreen();
      drawEntryPage(
        ui,
        surface,
        entry,
        entryData,
        pageIndex,
        0,
        ENTRY_ACTIONS,
        ENTRY_ACTION_COORDINATES,
        { isCaught: true, showArrowCursor: true },
      );
      frames.push({
        kind: "entry",
        slug: slugForEntryPage(entry, pageIndex),
        surface,
        speciesId: entry.species.id,
        pokedexNumber: entry.pokedexNumber,
        pageIndex,
        pageCount: entryData.pages.length,
      });
    }
  }

  return frames;
};

const renderRepresentativeEntryFrames = async (
  ui: HeadlessPokedexAuditUI,
): Promise<PokedexAuditFrame[]> => {
  const entries = getRenderableEntries();
  const pageRepresentatives = new Map<number, CachedDexEntry>();

  for (const entry of entries) {
    entry.entryData.pages.forEach((_page, pageIndex) => {
      if (!pageRepresentatives.has(pageIndex)) {
        pageRepresentatives.set(pageIndex, entry);
      }
    });
  }

  return Array.from(pageRepresentatives.entries())
    .sort(([leftPageIndex], [rightPageIndex]) => leftPageIndex - rightPageIndex)
    .map(([pageIndex, entry]) => {
      const surface = createBlankScreen();
      drawEntryPage(
        ui,
        surface,
        entry,
        entry.entryData,
        pageIndex,
        0,
        ENTRY_ACTIONS,
        ENTRY_ACTION_COORDINATES,
        { isCaught: true, showArrowCursor: true },
      );
      return {
        kind: "entry" as const,
        slug: `representative-entry-page-${pageIndex + 1}-${entry.species.id.toLowerCase()}`,
        surface,
        speciesId: entry.species.id,
        pokedexNumber: entry.pokedexNumber,
        pageIndex,
        pageCount: entry.entryData.pages.length,
      };
    });
};

const renderScreenFrames = (ui: HeadlessPokedexAuditUI): PokedexAuditFrame[] => {
  const frames: PokedexAuditFrame[] = [];
  const seenSet = new Set<number>();
  const caughtSet = new Set<number>();
  const { gameState, speciesEntries } = buildFullDexState();
  for (const entry of speciesEntries) {
    seenSet.add(entry.species.int_id);
    caughtSet.add(entry.species.int_id);
  }

  const pushScreen = (slug: string, draw: (surface: Surface) => void): void => {
    const surface = createBlankScreen();
    draw(surface);
    frames.push({ kind: "screen", slug, surface });
  };

  for (const mode of [DexMode.NEW, DexMode.OLD, DexMode.ABC]) {
    gameState.wram.wCurDexMode = mode;
    const [entries] = orderEntriesForMode(
      gameState,
      Object.fromEntries(speciesEntries.map((entry) => [entry.species.id, entry.species])),
      Object.fromEntries(speciesEntries.map((entry) => [entry.species.int_id, entry.species])),
      mode,
    );

    pushScreen(`main-${DexMode[mode].toLowerCase()}`, (surface) => {
      surface.blit(renderLayeredMainScreen(ui, entries, seenSet, caughtSet, mode), [0, 0]);
    });
  }

  pushScreen("search", (surface) => {
    drawSearchScreen(ui, surface, 0, [PokemonType.WATER, PokemonType.FLYING].map((type) => {
      const sequence = [
        PokemonType.NONE,
        PokemonType.NORMAL,
        PokemonType.FIRE,
        PokemonType.WATER,
        PokemonType.GRASS,
        PokemonType.ELECTRIC,
        PokemonType.ICE,
        PokemonType.FIGHTING,
        PokemonType.POISON,
        PokemonType.GROUND,
        PokemonType.FLYING,
        PokemonType.PSYCHIC_TYPE,
        PokemonType.BUG,
        PokemonType.ROCK,
        PokemonType.GHOST,
        PokemonType.DRAGON,
        PokemonType.DARK,
        PokemonType.STEEL,
      ];
      return sequence.indexOf(type);
    }) as [number, number]);
  });

  const searchResultEntries = speciesEntries.filter(
    (entry) => entry.species.type1 === PokemonType.WATER || entry.species.type2 === PokemonType.WATER,
  ).slice(0, 4);

  pushScreen("search-results", (surface) => {
    surface.blit(renderLayeredSearchResultsScreen(ui, searchResultEntries, seenSet, caughtSet, [3, 10]), [0, 0]);
  });

  pushScreen("options", (surface) => {
    drawOptionScreen(
      ui,
      surface,
      [DexMode.NEW, DexMode.OLD, DexMode.ABC, DexMode.UNOWN],
      0,
      null,
      true,
    );
  });

  pushScreen("unown", (surface) => {
    drawUnownModeScreen(
      ui,
      surface,
      Array.from({ length: NUM_UNOWN }, (_value, index) => index + 1),
      0,
      { word: "ANGRY", activeSpeciesId: "unown_a" },
    );
  });

  return frames;
};

export const renderPokedexAuditFrames = async (): Promise<PokedexAuditFrame[]> => {
  const ui = await HeadlessPokedexAuditUI.create();
  const entryFrames = await renderEntryFrames(ui);
  const screenFrames = renderScreenFrames(ui);
  return [...screenFrames, ...entryFrames];
};

export const renderRepresentativePokedexAuditFrames = async (): Promise<PokedexAuditFrame[]> => {
  const ui = await HeadlessPokedexAuditUI.create();
  const entryFrames = await renderRepresentativeEntryFrames(ui);
  const screenFrames = renderScreenFrames(ui);
  return [...screenFrames, ...entryFrames];
};

export const summarizePokedexAuditFrames = (frames: readonly PokedexAuditFrame[]): PokedexAuditSummary => {
  const entrySpecies = new Set(frames.filter((frame) => frame.kind === "entry").map((frame) => frame.speciesId));
  const entryPageCount = frames.filter((frame) => frame.kind === "entry").length;
  const screenCount = frames.filter((frame) => frame.kind === "screen").length;
  return {
    entrySpeciesCount: entrySpecies.size,
    entryPageCount,
    screenCount,
    totalFrameCount: frames.length,
  };
};

export const countRenderablePokedexPages = (): number => {
  return getRenderableEntries().reduce((total, entry) => {
    return total + entry.entryData.pages.length;
  }, 0);
};

export const buildPokedexSpeciesIndex = (): Record<string, PokemonSpecies> => {
  return Object.fromEntries(getRenderableEntries().map((entry) => [entry.species.id, entry.species]));
};
