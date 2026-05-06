// ASM mapping: pokecrystal_disassembly/engine/menus/party_menu.asm (tilemap layout + palettes).
import { TileRegion } from "@pokecrystal/core/ui/tile-layout";
import { TilemapSurface } from "@pokecrystal/core/ui/tilemap-surface";
import { Surface } from "@pokecrystal/core/ui/surface";
import { POKEMON_WORD } from "@pokecrystal/assets/content/text-constants";
import { Pokemon } from "@pokecrystal/core/core/models";
import { StatusCondition } from "@pokecrystal/core/core/enums";
import {
  MonMenuCategory,
  MonMenuEntry,
  MonMenuItem,
  MON_MENU_OPTIONS,
} from "@pokecrystal/core/core/enums/mon-menu";
import { PartyMenuQuality } from "@pokecrystal/core/core/enums/party-menu";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import type { Palette } from "@pokecrystal/core/ui/font-renderer";

type TileVariants = Record<number, Record<number, Surface>>;

const rgbKey = (r: number, g: number, b: number): string => `${r},${g},${b}`;

export const STATUS_LABELS: Record<string, string> = {
  "": "OK",
  [StatusCondition.POISON]: "PSN",
  [StatusCondition.SLEEP]: "SLP",
  [StatusCondition.PARALYSIS]: "PAR",
  [StatusCondition.BURN]: "BRN",
  [StatusCondition.FREEZE]: "FRZ",
  [StatusCondition.CONFUSION]: "CNF",
};

export const PARTY_LIST_REGION = new TileRegion(0, 0, 14, 14);
export const SUBMENU_REGION = new TileRegion(6, 0, 14, 18);
export const GIVE_TAKE_REGION = new TileRegion(12, 12, 8, 6);
export const DETAIL_REGION = new TileRegion(14, 0, 6, 18);
export const TEXTBOX_REGION = new TileRegion(0, 14, 18, 2);
export const TEXTBOX_BORDER_REGION = new TileRegion(
  TEXTBOX_REGION.left,
  TEXTBOX_REGION.top,
  TEXTBOX_REGION.width + 2,
  TEXTBOX_REGION.height + 2
);
export const POINTER_COLUMN = 0;
export const NAME_COLUMN = 3;
export const STATUS_COLUMN = 5;
export const LEVEL_COLUMN = 8;
export const COMPAT_COLUMN = 12;
export const HP_BAR_COLUMN = 11;
export const HP_DIGITS_COLUMN = 13;

// ASM row math (party_menu.asm).
const ASM_NAME_ROW_BASE = 1;
const ASM_STATUS_ROW_BASE = 2;
const ASM_ENTRY_ROW_HEIGHT = 2;
export const ENTRY_FIRST_ROW_TILE = ASM_NAME_ROW_BASE;
export const ENTRY_ROW_HEIGHT_TILES = ASM_ENTRY_ROW_HEIGHT;
export const CANCEL_COLUMN = NAME_COLUMN - 2;
export const HP_BAR_LENGTH_PX = 6 * TILE_SIZE;
export const NO_PARTY_TEXT = `You have no ${POKEMON_WORD}!`;
export const PAL_BG_TEXT = 0x07;
const HP_TILE_START = 0x60;
const HP_TILE_END = 0x6c;
const HP_TILE_IDS = Array.from({ length: HP_TILE_END - HP_TILE_START }, (_unused, index) => HP_TILE_START + index);

const GRAY_LEVELS = new Map<string, number>([
  [rgbKey(255, 255, 255), 0],
  [rgbKey(170, 170, 170), 1],
  [rgbKey(85, 85, 85), 2],
  [rgbKey(0, 0, 0), 3],
]);

const PARTY_ICON_GB_PALETTE: ReadonlyArray<ReadonlyArray<number>> = [
  [31, 31, 31],
  [18, 18, 18],
  [10, 10, 10],
  [0, 0, 0],
];

const HP_GB_PALETTES: Record<number, ReadonlyArray<ReadonlyArray<number>>> = {
  0: [
    [31, 31, 31],
    [30, 26, 15],
    [0, 23, 0],
    [0, 0, 0],
  ],
  1: [
    [31, 31, 31],
    [30, 26, 15],
    [31, 23, 0],
    [0, 0, 0],
  ],
  2: [
    [31, 31, 31],
    [30, 26, 15],
    [31, 0, 0],
    [0, 0, 0],
  ],
};

export const SUPPORTED_QUALITY_HANDLERS: Record<PartyMenuQuality, string> = {
  [PartyMenuQuality.NICKNAMES]: "_place_party_nicknames",
  [PartyMenuQuality.HP_BAR]: "_place_party_hp_bars",
  [PartyMenuQuality.HP_DIGITS]: "_place_party_hp_digits",
  [PartyMenuQuality.LEVEL]: "_place_party_levels",
  [PartyMenuQuality.STATUS]: "_place_party_status",
  [PartyMenuQuality.TMHM_COMPAT]: "_place_tmhm_compatibility",
  [PartyMenuQuality.EVO_STONE_COMPAT]: "_place_evo_stone_compatibility",
  [PartyMenuQuality.GENDER]: "_place_gender_labels",
  [PartyMenuQuality.MOBILE_SELECTION]: "_place_mobile_selection",
};

export const NUM_MON_SUBMENU_ITEMS = 8;
export const FIELD_MOVE_LOOKUP: Record<string, MonMenuEntry> = Object.fromEntries(
  MON_MENU_OPTIONS.filter((entry) => entry.category === MonMenuCategory.FIELD_MOVE).map((entry) => [
    String(entry.value),
    entry,
  ])
);
export const MENU_OPTION_LOOKUP: Partial<Record<MonMenuItem, MonMenuEntry>> = Object.fromEntries(
  MON_MENU_OPTIONS.filter((entry) => entry.category === MonMenuCategory.MENU_OPTION).map((entry) => [
    entry.item,
    entry,
  ])
);

export interface PartyEntry {
  index: number;
  pokemon: Pokemon;
}

export class MonMenuChoice {
  constructor(
    public readonly item: MonMenuItem,
    public readonly label: string,
    public readonly moveEntry: MonMenuEntry | null = null
  ) {}

  get isFieldMove(): boolean {
    return this.moveEntry !== null && this.moveEntry.category === MonMenuCategory.FIELD_MOVE;
  }
}

export const partyMenuNameRow = (rowIndex: number): number =>
  ASM_NAME_ROW_BASE + rowIndex * ASM_ENTRY_ROW_HEIGHT;

export const partyMenuStatusRow = (rowIndex: number): number =>
  ASM_STATUS_ROW_BASE + rowIndex * ASM_ENTRY_ROW_HEIGHT;

export const partyMenuCancelRow = (entryCount: number): number =>
  partyMenuNameRow(entryCount);

export class PartyMenuTilemap extends TilemapSurface {
  static readonly WIDTH = 20;
  static readonly HEIGHT = 18;

  constructor() {
    super(PartyMenuTilemap.WIDTH, PartyMenuTilemap.HEIGHT);
  }
}

export interface BitmapFontLike {
  paletteVariants?: (paletteOrder: ReadonlyArray<Palette>) => TileVariants;
  fontTiles?: Record<number, Surface>;
}

export const partyMenuTileset = (
  font: BitmapFontLike | Record<number, Surface>
): Record<number, Surface | Record<number, Surface>> => {
  const tiles: Record<number, Surface | Record<number, Surface>> = {};
  const paletteOrder = partyMenuPaletteOrder();

  let fontTiles: Record<number, Surface> = {};
  const fontLike = font as BitmapFontLike;
  if (typeof fontLike.paletteVariants === "function") {
    const variants = fontLike.paletteVariants(paletteOrder);
    Object.entries(variants).forEach(([tileId, paletteMap]) => {
      tiles[Number(tileId)] = paletteMap;
    });
    fontTiles = fontLike.fontTiles ?? {};
  } else {
    fontTiles = font as Record<number, Surface>;
  }

  const hpTemplates = hpTileTemplates(fontTiles);
  const palettes = hpPalettes();
  Object.entries(hpTemplates).forEach(([tileId, template]) => {
    const variants: Record<number, Surface> = {};
    Object.entries(palettes).forEach(([paletteIndexRaw, palette]) => {
      const paletteIndex = Number(paletteIndexRaw);
      const attrIndex = paletteIndex + 1;
      const tinted = tintHpTile(template, palette);
      variants[attrIndex] = tinted;
      if (paletteIndex === 0) {
        variants[paletteIndex] = tinted;
      }
    });
    tiles[Number(tileId)] = variants;
  });

  Object.entries(fontTiles).forEach(([tileId, surface]) => {
    const id = Number(tileId);
    if (tiles[id] === undefined) {
      tiles[id] = surface;
    }
  });
  return tiles;
};

const hpTileTemplates = (fontTiles: Record<number, Surface>): Record<number, Surface> => {
  const missing = HP_TILE_IDS.filter((tileId) => !(tileId in fontTiles));
  if (missing.length) {
    const missingIds = missing.map((tileId) => `0x${tileId.toString(16).toUpperCase()}`).join(", ");
    throw new Error(`Font tiles are missing HP bar glyphs: ${missingIds}`);
  }
  const templates: Record<number, Surface> = {};
  HP_TILE_IDS.forEach((tileId) => {
    templates[tileId] = fontTiles[tileId].copy();
  });
  return templates;
};

const hpPalettes = (): Record<number, ReadonlyArray<ReadonlyArray<number>>> => {
  const palettes: Record<number, ReadonlyArray<ReadonlyArray<number>>> = {};
  Object.entries(HP_GB_PALETTES).forEach(([index, colours]) => {
    palettes[Number(index)] = colours.map((colour) => toRgb(colour as ReadonlyArray<number>));
  });
  return palettes;
};

const toRgb = (colour: ReadonlyArray<number>): [number, number, number] => {
  const [r, g, b] = colour;
  return [gbc5To8(r), gbc5To8(g), gbc5To8(b)];
};

const tintHpTile = (template: Surface, palette: ReadonlyArray<ReadonlyArray<number>>): Surface => {
  const base = template.copy();
  const tinted = new Surface(base.width, base.height);
  for (let y = 0; y < base.height; y += 1) {
    for (let x = 0; x < base.width; x += 1) {
      const [r, g, b, a] = base.getAt(x, y);
      if (a === 0) {
        tinted.setAt(x, y, [0, 0, 0, 0]);
        continue;
      }
      const paletteIndex = GRAY_LEVELS.get(rgbKey(r, g, b)) ?? 0;
      const rgb = palette[paletteIndex] ?? palette[0];
      tinted.setAt(x, y, [rgb[0], rgb[1], rgb[2], a]);
    }
  }
  return tinted;
};

const partyMenuPaletteOrder = (): Palette[] => {
  const partyIcon: Palette = PARTY_ICON_GB_PALETTE.map((colour) => toRgb(colour));
  const hp = hpPalettes();
  return [
    partyIcon,
    hp[0] as Palette,
    hp[1] as Palette,
    hp[2] as Palette,
    partyIcon,
    partyIcon,
    partyIcon,
    partyIcon,
  ];
};
