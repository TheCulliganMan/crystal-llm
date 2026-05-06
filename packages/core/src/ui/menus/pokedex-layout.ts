// ASM mapping: engine/pokedex/pokedex.asm list + search layout helpers.
import { DexMode } from "../../core/enums/pokedex";
import { PokemonType } from "../../core/enums/pokemon";
import { TILE_SIZE } from "../../engine/world/tile";
import { Surface } from "../surface";
import { buildDefaultCharMap } from "../text/glyph-map";
import { renderFontText } from "../text/render-font";
import { ensurePokedexTiles, requirePokedexTile, type PokedexFontSource } from "./pokedex-assets";

export const LIST_WINDOW_LENGTH = 7;
export const LIST_SECTION_X = 0;
export const LIST_SECTION_Y = 0;
export const LIST_SECTION_WIDTH = 11;
const BACKGROUND_TILE_INDEX = 0x32;
export const SCREEN_WIDTH_TILES = 20;
export const SCREEN_HEIGHT_TILES = 18;

export const DETAIL_SECTION_X = LIST_SECTION_WIDTH + 1;
export const DETAIL_SECTION_Y = LIST_SECTION_Y;
export const DETAIL_SECTION_WIDTH = 8;
export const DETAIL_SECTION_HEIGHT = 12;

export const POKEDEX_TEXT_COLOR: [number, number, number] = [255, 255, 255];

export const SELECT_PROMPT = "SELECT \u25b6 OPTION";
export const BACKGROUND_COLOR: [number, number, number] = [240, 248, 255];
const SCROLL_TILES_OLD: [number, number, number] = [0x66, 0x67, 0x68];
const SCROLL_TILES_NEW: [number, number, number] = [0x50, 0x51, 0x52];
export const SEARCH_WINDOW_WIDTH = 14;
export const SEARCH_WINDOW_HEIGHT = 18;
export const TYPE_SELECTOR_ARROW_LEFT = 0x3d;
export const TYPE_SELECTOR_ARROW_RIGHT = 0x3e;
export const TYPE_SELECTOR_NAME_WIDTH = 8;
export const SEARCH_RESULTS_TOP_WINDOW_HEIGHT = 11;
export const SEARCH_RESULTS_BOTTOM_WINDOW_HEIGHT = 7;
const SEARCH_TYPE_NOT_FOUND_TOP_ROW = 12;
const SEARCH_TYPE_NOT_FOUND_LINE_1 = "The specified type";
const SEARCH_TYPE_NOT_FOUND_LINE_2 = "was not found.";

export const SEARCH_TYPE_SEQUENCE: PokemonType[] = [
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

// ASM: data/types/search_strings.asm::PokedexTypeSearchStrings
const SEARCH_TYPE_STRINGS = [
  "  ----  ",
  " NORMAL ",
  "  FIRE  ",
  " WATER  ",
  " GRASS  ",
  "ELECTRIC",
  "  ICE   ",
  "FIGHTING",
  " POISON ",
  " GROUND ",
  " FLYING ",
  "PSYCHIC ",
  "  BUG   ",
  "  ROCK  ",
  " GHOST  ",
  " DRAGON ",
  "  DARK  ",
  " STEEL  ",
] as const;

const SEARCH_MENU_ITEMS = ["BEGIN SEARCH!!", "CANCEL"] as const;
const CHAR_MAP = buildDefaultCharMap();

const TYPE_DISPLAY_OVERRIDES: Partial<Record<PokemonType, string>> = {
  [PokemonType.PSYCHIC_TYPE]: "PSYCHIC",
  [PokemonType.CURSE_TYPE]: "CURSE",
  [PokemonType.UNKNOWN]: "????",
  [PokemonType.NONE]: "----",
};
const BORDER_TILES = [0x33, 0x34, 0x35, 0x36, 0x7f, 0x37, 0x38, 0x39, 0x3a] as const;
export const MAIN_LIST_HEIGHT = 17;
const LIST_DECORATION = [0x3f, 0x40] as const;

export type PokedexLayoutEntry = {
  pokedexNumber: number;
  species: { id: string; int_id?: number };
};

const resolveEntrySpeciesId = (entry: PokedexLayoutEntry): number => {
  const speciesIntId = Number(entry.species.int_id);
  if (Number.isInteger(speciesIntId) && speciesIntId > 0) {
    return speciesIntId;
  }
  return entry.pokedexNumber;
};

type PokedexFont = {
  render_text?: (
    text: string,
    x: number,
    y: number,
    surface: Surface | import("@pokecrystal/core/ui/font-renderer").SurfaceLike,
    options?: import("@pokecrystal/core/ui/font-renderer").RenderTextOptions
  ) => void;
  renderText: (
    text: string,
    x: number,
    y: number,
    surface: Surface | import("@pokecrystal/core/ui/font-renderer").SurfaceLike,
    options?: boolean | import("@pokecrystal/core/ui/font-renderer").RenderTextOptions
  ) => void;
  fontTiles?: Record<number, Surface>;
  font_tiles?: Record<number, Surface>;
};

export type PokedexLayoutUI = {
  font: PokedexFont;
} & PokedexFontSource;

export const typeDisplayName = (pokemonType: PokemonType): string => {
  const override = TYPE_DISPLAY_OVERRIDES[pokemonType];
  if (override) {
    return override;
  }
  return pokemonType.replace("_TYPE", "").replace(/_/g, " ");
};

const searchTypeString = (typeIndex: number): string => {
  return SEARCH_TYPE_STRINGS[typeIndex] ?? SEARCH_TYPE_STRINGS[0];
};

const opaqueWhiteGlyphTile = (tile: Surface): Surface => {
  const [width, height] = tile.get_size();
  const opaque = new Surface(width, height);
  opaque.fill([0, 0, 0, 255]);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const [_r, _g, _b, a] = tile.get_at([x, y]);
      if (a > 0) {
        opaque.set_at([x, y], [255, 255, 255, 255]);
      }
    }
  }
  return opaque;
};

const renderOpaquePanelString = (
  ui: PokedexLayoutUI,
  screen: Surface,
  text: string,
  x: number,
  y: number,
): void => {
  for (let index = 0; index < text.length; index += 1) {
    const char = text[index] ?? " ";
    const tileIndex = char === " " ? 0x7f : CHAR_MAP[char];
    if (tileIndex === undefined) {
      throw new Error(`Character ${JSON.stringify(char)} is not available in the Pokedex font.`);
    }
    const tile = requireFontTile(ui, tileIndex);
    screen.blit(char === " " ? tile : opaqueWhiteGlyphTile(tile), [x + index * TILE_SIZE, y]);
  }
};

export const drawPokedexListWindow = (
  ui: PokedexLayoutUI,
  screen: Surface,
  listSectionX: number,
  listSectionY: number,
  windowHeight: number,
  dexMode: DexMode,
  options?: { drawPrompts?: boolean; origin?: [number, number] }
): void => {
  const originX = options?.origin?.[0] ?? 0;
  const originY = options?.origin?.[1] ?? 0;
  const listX = originX + listSectionX * TILE_SIZE;
  const listY = originY + listSectionY * TILE_SIZE;
  drawPokedexListFrame(ui, screen, listX, listY, dexMode, windowHeight);
  if (options?.drawPrompts !== false) {
    drawSelectStartPrompts(ui, screen, originX);
  }
};

export const drawPokedexList = (
  ui: PokedexLayoutUI,
  screen: Surface,
  entries: PokedexLayoutEntry[],
  cursorIndex: number,
  scrollOffset: number,
  seenSet: Iterable<number>,
  caughtSet: Iterable<number>,
  dexMode: DexMode,
  windowLength: number = LIST_WINDOW_LENGTH,
  options?: {
    listSectionX?: number;
    listSectionY?: number;
    windowHeight?: number | null;
    drawWindow?: boolean;
    windowPrompts?: boolean;
    originOffset?: [number, number];
  },
): void => {
  const listSectionX = options?.listSectionX ?? LIST_SECTION_X;
  const listSectionY = options?.listSectionY ?? LIST_SECTION_Y;
  const originOffset = options?.originOffset ?? [0, 0];
  const listX = originOffset[0] + listSectionX * TILE_SIZE;
  const listY = originOffset[1] + listSectionY * TILE_SIZE;
  const visibleHeight = options?.windowHeight ?? windowLength;
  if (options?.drawWindow !== false) {
    drawPokedexListWindow(ui, screen, listSectionX, listSectionY, visibleHeight, dexMode, {
      drawPrompts: options?.windowPrompts !== false,
      origin: originOffset,
    });
  }

  const total = entries.length;
  if (!total) {
    renderFontText(ui.font, "No entries", listX + TILE_SIZE, listY + 2 * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
    return;
  }

  const maxScroll = Math.max(0, total - windowLength);
  const scroll = Math.max(0, Math.min(scrollOffset, maxScroll));
  const visible = entries.slice(scroll, scroll + windowLength);

  const nameX = listX + TILE_SIZE;
  const numberX = listX;
  const caughtX = listX;
  const textWidth = (LIST_SECTION_WIDTH - 1) * TILE_SIZE;
  const seen = new Set(seenSet);
  const caught = new Set(caughtSet);
  for (let row = 0; row < windowLength; row += 1) {
    if (row >= visible.length) {
      continue;
    }
    const entry = visible[row];
    const speciesId = resolveEntrySpeciesId(entry);
    const entryRow = listY + (2 + row * 2) * TILE_SIZE;
    const numberRow = entryRow - TILE_SIZE;
    // ASM mapping: Pokedex_PrintListing -> Pokedex_CheckSeen/Pokedex_CheckCaught
    // checks use the species id from wPokedexOrder, not a list index.
    const seenFlag = seen.has(speciesId);
    if (dexMode === DexMode.OLD) {
      renderFontText(ui.font, `${entry.pokedexNumber}`.padStart(3, "0"), numberX, numberRow, screen, { color: POKEDEX_TEXT_COLOR });
    }
    const displayName = seenFlag ? entry.species.id : "-----";
    renderFontText(ui.font, displayName, nameX, entryRow, screen, { textWidth, color: POKEDEX_TEXT_COLOR });
    if (seenFlag && caught.has(speciesId)) {
      screen.blit(requireFontTile(ui, 0x4f), [caughtX, entryRow]);
    }
  }
};

const requireFontTile = (ui: PokedexLayoutUI, tileIndex: number): Surface => {
  ensurePokedexTiles(ui);
  return requirePokedexTile(ui, tileIndex);
};

export const drawMainCounters = (
  ui: PokedexLayoutUI,
  screen: Surface,
  seenCount: number,
  ownCount: number,
): void => {
  const baseX = TILE_SIZE;
  const seenY = 11 * TILE_SIZE;
  const ownY = 14 * TILE_SIZE;
  renderFontText(ui.font, "SEEN", baseX, seenY, screen, { color: POKEDEX_TEXT_COLOR });
  renderFontText(ui.font, `${seenCount}`.padStart(3, " "), 5 * TILE_SIZE, seenY + TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  renderFontText(ui.font, "OWN", baseX, ownY, screen, { color: POKEDEX_TEXT_COLOR });
  renderFontText(ui.font, `${ownCount}`.padStart(3, " "), 5 * TILE_SIZE, ownY + TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
};

const scrollTilesForMode = (dexMode: DexMode): [number, number, number] => {
  return dexMode === DexMode.OLD ? SCROLL_TILES_OLD : SCROLL_TILES_NEW;
};

export const drawSelectStartPrompts = (
  ui: PokedexLayoutUI,
  screen: Surface,
  originX: number = 0,
): void => {
  const promptY = 17 * TILE_SIZE;
  fillTileBlock(ui, screen, BACKGROUND_TILE_INDEX, originX, promptY, LIST_SECTION_WIDTH + 1, 1);
  renderTileString(ui, screen, [0x3c, 0x3b, 0x41, 0x42, 0x43, 0x4b, 0x4c, 0x4d, 0x4e, 0x3c], originX, promptY);
};

export const drawSearchScreen = (
  ui: PokedexLayoutUI,
  screen: Surface,
  searchCursor: number,
  typeIndexes: [number, number],
  options?: { showArrowCursor?: boolean },
): void => {
  fillTileBlock(ui, screen, BACKGROUND_TILE_INDEX, 0, 0, SCREEN_WIDTH_TILES, SCREEN_HEIGHT_TILES);
  drawPokedexBorder(ui, screen, 0, 2 * TILE_SIZE, 18, 14);
  const title = " SEARCH ";
  screen.blit(requireFontTile(ui, 0x3b), [0, TILE_SIZE]);
  renderFontText(ui.font, title, TILE_SIZE, TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  screen.blit(requireFontTile(ui, 0x3c), [(1 + title.length) * TILE_SIZE, TILE_SIZE]);

  const leftArrow = requireFontTile(ui, TYPE_SELECTOR_ARROW_LEFT);
  const rightArrow = requireFontTile(ui, TYPE_SELECTOR_ARROW_RIGHT);
  const arrowX = 8 * TILE_SIZE;
  const textX = arrowX + TILE_SIZE;
  const rightArrowX = textX + TYPE_SELECTOR_NAME_WIDTH * TILE_SIZE;
  const showArrow = options?.showArrowCursor !== false;

  [4, 6].forEach((labelRow, rowIndex) => {
    if (searchCursor === rowIndex) {
      renderFontText(ui.font, showArrow ? "\u25b6" : " ", 2 * TILE_SIZE, labelRow * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
    }
    renderFontText(ui.font, `TYPE${rowIndex + 1}`, 3 * TILE_SIZE, labelRow * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
    const typeName = searchTypeString(typeIndexes[rowIndex]);
    renderOpaquePanelString(ui, screen, typeName, textX, labelRow * TILE_SIZE);
    screen.blit(leftArrow, [arrowX, labelRow * TILE_SIZE]);
    screen.blit(rightArrow, [rightArrowX, labelRow * TILE_SIZE]);
  });

  SEARCH_MENU_ITEMS.forEach((label, index) => {
    const row = 13 + index;
    const cursorChar = searchCursor === 2 + index ? (showArrow ? "\u25b6" : " ") : " ";
    renderFontText(ui.font, cursorChar, 2 * TILE_SIZE, row * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
    renderFontText(ui.font, label, 3 * TILE_SIZE, row * TILE_SIZE, screen, { color: POKEDEX_TEXT_COLOR });
  });
};

// ASM: engine/pokedex/pokedex_3.asm::DrawPokedexSearchResultsWindow.
export const drawSearchResultsWindow = (
  ui: PokedexLayoutUI,
  screen: Surface,
  entries: PokedexLayoutEntry[],
  cursorIndex: number,
  scrollOffset: number,
  typeIndexes: [number, number],
  seenSet: Iterable<number>,
  caughtSet: Iterable<number>,
  dexMode: DexMode,
  windowLength: number = LIST_WINDOW_LENGTH,
  options?: { originOffset?: [number, number] },
): void => {
  const originOffset = options?.originOffset ?? [0, 0];
  drawSearchResultsFrame(ui, screen, originOffset);
  drawPokedexList(
    ui,
    screen,
    entries,
    cursorIndex,
    scrollOffset,
    seenSet,
    caughtSet,
    dexMode,
    windowLength,
    {
      listSectionY: 0,
      drawWindow: false,
      windowPrompts: false,
      originOffset,
    },
  );
  const [originX, originY] = originOffset;
  renderOpaquePanelString(ui, screen, "ESULTS", originX, originY + 12 * TILE_SIZE);
  renderOpaquePanelString(ui, screen, "D!", originX, originY + 14 * TILE_SIZE);

  // ASM: engine/pokedex/pokedex.asm::Pokedex_PlaceSearchResultsTypeStrings.
  const type1 = SEARCH_TYPE_SEQUENCE[typeIndexes[0]] ?? PokemonType.NONE;
  const type2 = SEARCH_TYPE_SEQUENCE[typeIndexes[1]] ?? PokemonType.NONE;
  renderOpaquePanelString(ui, screen, searchTypeString(typeIndexes[0]), originX, originY + 14 * TILE_SIZE);
  const showSecondType = type2 !== PokemonType.NONE && type2 !== type1;
  if (showSecondType) {
    const type2X = originX + 2 * TILE_SIZE;
    const type2Y = originY + 15 * TILE_SIZE;
    renderOpaquePanelString(ui, screen, "/", originX + TILE_SIZE, type2Y);
    renderOpaquePanelString(ui, screen, searchTypeString(typeIndexes[1]), type2X, type2Y);
  }
};

// ASM: engine/pokedex/pokedex.asm::Pokedex_DisplayTypeNotFoundMessage.
export const drawSearchTypeNotFoundMessage = (ui: PokedexLayoutUI, screen: Surface): void => {
  drawPokedexBorder(ui, screen, 0, SEARCH_TYPE_NOT_FOUND_TOP_ROW * TILE_SIZE, 18, 4);
  renderFontText(ui.font, SEARCH_TYPE_NOT_FOUND_LINE_1, TILE_SIZE, (SEARCH_TYPE_NOT_FOUND_TOP_ROW + 2) * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
  });
  renderFontText(ui.font, SEARCH_TYPE_NOT_FOUND_LINE_2, TILE_SIZE, (SEARCH_TYPE_NOT_FOUND_TOP_ROW + 3) * TILE_SIZE, screen, {
    color: POKEDEX_TEXT_COLOR,
  });
};

const tileCoords = (originX: number, originY: number, tileX: number, tileY: number): [number, number] => {
  return [originX + tileX * TILE_SIZE, originY + tileY * TILE_SIZE];
};

export const fillTileBlock = (
  ui: PokedexLayoutUI,
  screen: Surface,
  tileIndex: number,
  originX: number,
  originY: number,
  widthTiles: number,
  heightTiles: number,
): void => {
  const tile = requireFontTile(ui, tileIndex);
  for (let row = 0; row < heightTiles; row += 1) {
    for (let col = 0; col < widthTiles; col += 1) {
      screen.blit(tile, tileCoords(originX, originY, col, row));
    }
  }
};

export const renderTileString = (
  ui: PokedexLayoutUI,
  screen: Surface,
  tiles: number[],
  x: number,
  y: number,
): void => {
  tiles.forEach((tileIndex, offset) => {
    const tile = requireFontTile(ui, tileIndex);
    screen.blit(tile, [x + offset * TILE_SIZE, y]);
  });
};

export const drawPokedexBorder = (
  ui: PokedexLayoutUI,
  screen: Surface,
  originX: number,
  originY: number,
  interiorWidth: number,
  interiorHeight: number,
): void => {
  const [topLeft, topEdge, topRight, leftEdge, center, rightEdge, bottomLeft, bottomEdge, bottomRight] =
    BORDER_TILES.map((index) => requireFontTile(ui, index));
  const widthTiles = interiorWidth + 2;
  const heightTiles = interiorHeight + 2;

  screen.blit(topLeft, [originX, originY]);
  for (let col = 1; col < widthTiles - 1; col += 1) {
    screen.blit(topEdge, tileCoords(originX, originY, col, 0));
  }
  screen.blit(topRight, tileCoords(originX, originY, widthTiles - 1, 0));

  for (let row = 1; row < heightTiles - 1; row += 1) {
    const y = originY + row * TILE_SIZE;
    screen.blit(leftEdge, [originX, y]);
    for (let col = 1; col < widthTiles - 1; col += 1) {
      screen.blit(center, tileCoords(originX, originY, col, row));
    }
    screen.blit(rightEdge, tileCoords(originX, originY, widthTiles - 1, row));
  }

  const bottomY = originY + (heightTiles - 1) * TILE_SIZE;
  screen.blit(bottomLeft, [originX, bottomY]);
  for (let col = 1; col < widthTiles - 1; col += 1) {
    screen.blit(bottomEdge, tileCoords(originX, originY, col, heightTiles - 1));
  }
  screen.blit(bottomRight, tileCoords(originX, originY, widthTiles - 1, heightTiles - 1));
};

const drawPokedexListFrame = (
  ui: PokedexLayoutUI,
  screen: Surface,
  originX: number,
  originY: number,
  dexMode: DexMode,
  windowHeight: number,
): void => {
  const [topTile, middleTile, bottomTile] = scrollTilesForMode(dexMode);
  const interiorHeight = Math.min(MAIN_LIST_HEIGHT - 2, windowHeight * 2 + 1);
  fillTileBlock(ui, screen, 0x7f, originX, originY + TILE_SIZE, LIST_SECTION_WIDTH, interiorHeight);
  fillTileBlock(
    ui,
    screen,
    BACKGROUND_TILE_INDEX,
    originX,
    originY + MAIN_LIST_HEIGHT * TILE_SIZE,
    LIST_SECTION_WIDTH + 1,
    1,
  );
  fillRow(ui, screen, 0x34, originX, originY, LIST_SECTION_WIDTH);
  fillRow(ui, screen, 0x39, originX, originY + (MAIN_LIST_HEIGHT - 1) * TILE_SIZE, LIST_SECTION_WIDTH);
  blitDecoration(ui, screen, LIST_DECORATION[0], originX, originY, 5, 0);
  blitDecoration(ui, screen, LIST_DECORATION[1], originX, originY, 5, MAIN_LIST_HEIGHT - 1);
  drawScrollColumnSegment(
    ui,
    screen,
    topTile,
    middleTile,
    bottomTile,
    originX + LIST_SECTION_WIDTH * TILE_SIZE,
    originY,
    15,
  );
};

const drawScrollColumnSegment = (
  ui: PokedexLayoutUI,
  screen: Surface,
  topIndex: number,
  middleIndex: number,
  bottomIndex: number,
  columnX: number,
  originY: number,
  middleHeight: number,
): void => {
  const topTile = requireFontTile(ui, topIndex);
  const middleTile = requireFontTile(ui, middleIndex);
  const bottomTile = requireFontTile(ui, bottomIndex);
  screen.blit(topTile, [columnX, originY]);
  for (let row = 0; row < middleHeight; row += 1) {
    screen.blit(middleTile, [columnX, originY + (row + 1) * TILE_SIZE]);
  }
  screen.blit(bottomTile, [columnX, originY + (middleHeight + 1) * TILE_SIZE]);
};

const fillRow = (
  ui: PokedexLayoutUI,
  screen: Surface,
  tileIndex: number,
  originX: number,
  originY: number,
  widthTiles: number,
): void => {
  const tile = requireFontTile(ui, tileIndex);
  for (let col = 0; col < widthTiles; col += 1) {
    screen.blit(tile, tileCoords(originX, originY, col, 0));
  }
};

const blitDecoration = (
  ui: PokedexLayoutUI,
  screen: Surface,
  tileIndex: number,
  originX: number,
  originY: number,
  tileX: number,
  tileY: number,
): void => {
  screen.blit(requireFontTile(ui, tileIndex), tileCoords(originX, originY, tileX, tileY));
};

const drawSearchResultsFrame = (
  ui: PokedexLayoutUI,
  screen: Surface,
  origin: [number, number],
): void => {
  const [originX, originY] = origin;
  const scrollX = originX + LIST_SECTION_WIDTH * TILE_SIZE;
  const topWindowHeight = SEARCH_RESULTS_TOP_WINDOW_HEIGHT;
  const bottomWindowHeight = SEARCH_RESULTS_BOTTOM_WINDOW_HEIGHT;
  const topBottomY = originY + (topWindowHeight - 1) * TILE_SIZE;
  const bottomOriginY = originY + topWindowHeight * TILE_SIZE;
  const bottomBottomY = bottomOriginY + (bottomWindowHeight - 1) * TILE_SIZE;

  fillRow(ui, screen, 0x34, originX, originY, LIST_SECTION_WIDTH);
  fillRow(ui, screen, 0x39, originX, topBottomY, LIST_SECTION_WIDTH);
  blitDecoration(ui, screen, LIST_DECORATION[0], originX, originY, 5, 0);
  blitDecoration(ui, screen, LIST_DECORATION[1], originX, originY, 5, topWindowHeight - 1);
  drawScrollColumnSegment(ui, screen, 0x66, 0x67, 0x68, scrollX, originY, topWindowHeight - 2);
  fillRow(ui, screen, 0x34, originX, bottomOriginY, LIST_SECTION_WIDTH);
  fillRow(ui, screen, 0x39, originX, bottomBottomY, LIST_SECTION_WIDTH);
  drawScrollColumnSegment(
    ui,
    screen,
    0x66,
    0x67,
    0x68,
    scrollX,
    bottomOriginY,
    bottomWindowHeight - 2,
  );
  fillTileBlock(ui, screen, 0x7f, originX, originY + TILE_SIZE, LIST_SECTION_WIDTH, topWindowHeight - 2);
  fillTileBlock(ui, screen, 0x7f, originX, bottomOriginY + TILE_SIZE, LIST_SECTION_WIDTH, bottomWindowHeight - 2);
};
