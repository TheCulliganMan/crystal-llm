// ASM: engine/pokegear/pokegear.asm background load and tilemap state.
import fs from "fs";
import { assetExists } from "../../core/asset-manifest";
import { readJsonAssetSync } from "../../core/asset-reader";
import { PlayerGender } from "../../core/enums";
import { LANDMARK_FAST_SHIP } from "../../core/constants";
import { getWorldMapLocation } from "../../core/home";
import { decompress } from "../../core/lz";
import { getAssetsRoot } from "../../core/paths";
import { joinPath } from "../../core/path-utils";
import { TilemapSurface } from "../tilemap-surface";
import { Surface } from "../surface";
import { gbc5To8 } from "../../core/gbc-colors";
import { NpcPaletteManager } from "../../engine/world/overworld/palette";

const { PNG } = require("pngjs") as any;

const SCREEN_TILES_W = 20;
const SCREEN_TILES_H = 18;
const TILE_SIZE = 8;
const TILE_COUNT = SCREEN_TILES_W * SCREEN_TILES_H;
const GB_GRAY_LEVELS = [0, 85, 170, 255] as const;
const POKEGEAR_PAL = "pokegear.pal";
const POKEGEAR_F_PAL = "pokegear_f.pal";
const POKEGEAR_TOWN_MAP_PALETTE_MAP = "pokegear_town_map_palette_map.json";
const POKEGEAR_INIT_WX = 0x07;
const POKEGEAR_INIT_WY = 0x90;

const TOKEN_TO_LABEL: Record<string, string> = {
  BORDER: "border",
  EARTH: "earth",
  MOUNTAIN: "mountain",
  CITY: "city",
  POI: "point_of_interest",
  POI_MTN: "mountain_point_of_interest",
};

type Palette = Array<[number, number, number]>;

const normalizeGender = (value: PlayerGender | number | null | undefined): PlayerGender => {
  if (value === null || value === undefined) {
    return PlayerGender.MALE;
  }
  // If it's already a valid enum value (number), return it.
  if (typeof value === "number") {
    return value === PlayerGender.FEMALE ? PlayerGender.FEMALE : PlayerGender.MALE;
  }
  return PlayerGender.MALE;
};


const readBytes = (filePath: string): Buffer => {
  const data = fs.readFileSync(filePath);
  if (filePath.endsWith(".lz")) {
    return Buffer.from(decompress(data));
  }
  return data;
};

const loadTilesetBytes = (assetDir: string, stem: string): Buffer | null => {
  const candidates = [
    joinPath(assetDir, `${stem}.2bpp.lz`),
    joinPath(assetDir, `${stem}.2bpp`),
  ];
  for (const candidate of candidates) {
    if (!assetExists(candidate)) {
      continue;
    }
    return readBytes(candidate);
  }
  return null;
};

const decode2bppLevels = (data: Buffer): number[][] => {
  if (data.length % 16 !== 0) {
    throw new Error("2bpp payload must be aligned to 16-byte tiles");
  }
  const tiles: number[][] = [];
  const tileCount = data.length / 16;
  for (let tileIndex = 0; tileIndex < tileCount; tileIndex += 1) {
    const base = tileIndex * 16;
    const levels: number[] = [];
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const plane0 = data[base + row * 2];
      const plane1 = data[base + row * 2 + 1];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const idx = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
        levels.push(idx);
      }
    }
    tiles.push(levels);
  }
  return tiles;
};

const decode2bppTiles = (data: Buffer): Surface[] => {
  const tiles: Surface[] = [];
  const levels = decode2bppLevels(data);
  for (const tileLevels of levels) {
    const surface = new Surface(TILE_SIZE, TILE_SIZE);
    for (let idx = 0; idx < tileLevels.length; idx += 1) {
      const x = idx % TILE_SIZE;
      const y = Math.floor(idx / TILE_SIZE);
      const gray = GB_GRAY_LEVELS[tileLevels[idx] as 0 | 1 | 2 | 3];
      surface.setAt(x, y, [gray, gray, gray, 255]);
    }
    tiles.push(surface);
  }
  return tiles;
};

const decodePngTiles = (filePath: string): Surface[] => {
  const image = PNG.sync.read(fs.readFileSync(filePath));
  if (image.width % TILE_SIZE !== 0 || image.height % TILE_SIZE !== 0) {
    throw new Error(`PNG tileset ${filePath} must align to ${TILE_SIZE}x${TILE_SIZE} tiles.`);
  }
  const tiles: Surface[] = [];
  const tilesWide = image.width / TILE_SIZE;
  const tilesHigh = image.height / TILE_SIZE;
  for (let tileY = 0; tileY < tilesHigh; tileY += 1) {
    for (let tileX = 0; tileX < tilesWide; tileX += 1) {
      const surface = new Surface(TILE_SIZE, TILE_SIZE);
      for (let y = 0; y < TILE_SIZE; y += 1) {
        for (let x = 0; x < TILE_SIZE; x += 1) {
          const sourceX = tileX * TILE_SIZE + x;
          const sourceY = tileY * TILE_SIZE + y;
          const offset = (sourceY * image.width + sourceX) * 4;
          const r = image.data[offset] ?? 0;
          const g = image.data[offset + 1] ?? 0;
          const b = image.data[offset + 2] ?? 0;
          const a = image.data[offset + 3] ?? 255;
          const gray = Math.round((r + g + b) / 3);
          const level = Math.round(((255 - gray) / 255) * 3) as 0 | 1 | 2 | 3;
          const normalized = GB_GRAY_LEVELS[level];
          surface.setAt(x, y, [normalized, normalized, normalized, a]);
        }
      }
      tiles.push(surface);
    }
  }
  return tiles;
};

const decodePngTileLevels = (filePath: string, maxLevel: 1 | 3): number[][] => {
  const image = PNG.sync.read(fs.readFileSync(filePath));
  if (image.width % TILE_SIZE !== 0 || image.height % TILE_SIZE !== 0) {
    throw new Error(`PNG tileset ${filePath} must align to ${TILE_SIZE}x${TILE_SIZE} tiles.`);
  }
  const tiles: number[][] = [];
  const tilesWide = image.width / TILE_SIZE;
  const tilesHigh = image.height / TILE_SIZE;
  for (let tileY = 0; tileY < tilesHigh; tileY += 1) {
    for (let tileX = 0; tileX < tilesWide; tileX += 1) {
      const levels: number[] = [];
      for (let y = 0; y < TILE_SIZE; y += 1) {
        for (let x = 0; x < TILE_SIZE; x += 1) {
          const sourceX = tileX * TILE_SIZE + x;
          const sourceY = tileY * TILE_SIZE + y;
          const offset = (sourceY * image.width + sourceX) * 4;
          const r = image.data[offset] ?? 0;
          const g = image.data[offset + 1] ?? 0;
          const b = image.data[offset + 2] ?? 0;
          const gray = Math.round((r + g + b) / 3);
          levels.push(Math.round(((255 - gray) / 255) * maxLevel));
        }
      }
      tiles.push(levels);
    }
  }
  return tiles;
};

const load1bppTileLevels = (assetDir: string, stem: string): number[][] => {
  const rawPath = joinPath(assetDir, `${stem}.1bpp`);
  if (assetExists(rawPath)) {
    return decode1bppLevels(readBytes(rawPath));
  }
  const lzPath = joinPath(assetDir, `${stem}.1bpp.lz`);
  if (assetExists(lzPath)) {
    return decode1bppLevels(readBytes(lzPath));
  }
  const pngPath = joinPath(assetDir, `${stem}.png`);
  if (assetExists(pngPath)) {
    return decodePngTileLevels(pngPath, 1);
  }
  throw new Error(`Missing 1bpp tileset for ${stem}: tried ${rawPath}, ${lzPath}, ${pngPath}`);
};

const load2bppTileLevels = (assetDir: string, stem: string): number[][] => {
  const rawPath = joinPath(assetDir, `${stem}.2bpp`);
  if (assetExists(rawPath)) {
    return decode2bppLevels(readBytes(rawPath));
  }
  const lzPath = joinPath(assetDir, `${stem}.2bpp.lz`);
  if (assetExists(lzPath)) {
    return decode2bppLevels(readBytes(lzPath));
  }
  const pngPath = joinPath(assetDir, `${stem}.png`);
  if (assetExists(pngPath)) {
    return decodePngTileLevels(pngPath, 3);
  }
  throw new Error(`Missing 2bpp tileset for ${stem}: tried ${rawPath}, ${lzPath}, ${pngPath}`);
};

const loadTilesetSurfaces = (assetDir: string, stem: string): Surface[] => {
  const bytes = loadTilesetBytes(assetDir, stem);
  if (bytes) {
    return decode2bppTiles(bytes);
  }
  const pngPath = joinPath(assetDir, `${stem}.png`);
  if (assetExists(pngPath)) {
    return decodePngTiles(pngPath);
  }
  throw new Error(
    `Missing tileset for ${stem}: tried ${[
      joinPath(assetDir, `${stem}.2bpp.lz`),
      joinPath(assetDir, `${stem}.2bpp`),
      pngPath,
    ].join(", ")}`
  );
};

const decode1bppLevels = (data: Buffer): number[][] => {
  if (data.length % 8 !== 0) {
    throw new Error("1bpp payload must be aligned to 8-byte tiles");
  }
  const tiles: number[][] = [];
  const tileCount = data.length / 8;
  for (let tileIndex = 0; tileIndex < tileCount; tileIndex += 1) {
    const base = tileIndex * 8;
    const levels: number[] = [];
    for (let row = 0; row < TILE_SIZE; row += 1) {
      const byte = data[base + row];
      for (let col = 0; col < TILE_SIZE; col += 1) {
        const bit = 7 - col;
        const set = (byte >> bit) & 1;
        levels.push(set ? 3 : 0);
      }
    }
    tiles.push(levels);
  }
  return tiles;
};

const renderTileFromLevels = (levels: number[], palette: Palette): Surface => {
  const surface = new Surface(TILE_SIZE, TILE_SIZE);
  for (let idx = 0; idx < levels.length; idx += 1) {
    const x = idx % TILE_SIZE;
    const y = Math.floor(idx / TILE_SIZE);
    const [r, g, b] = palette[levels[idx]];
    surface.setAt(x, y, [r, g, b, 255]);
  }
  return surface;
};

const sanitizeLabel = (label: string): string => {
  let result = label.split("(", 1)[0].toLowerCase();
  const replacements: Record<string, string> = {
    "(": " ",
    ")": " ",
    "-": " ",
    "/": " ",
    ",": " ",
  };
  for (const [key, value] of Object.entries(replacements)) {
    result = result.split(key).join(value);
  }
  result = result.replace(/\s{2,}/g, " ").trim();
  return result
    .split(" ")
    .filter((part) => part)
    .join("_");
};

const scaleComponent = (value: number): number => gbc5To8(value);

const parsePaletteBank = (palPath: string): Record<string, Palette> => {
  if (!assetExists(palPath)) {
    throw new Error(`Missing palette file: ${palPath}`);
  }
  const lines = fs.readFileSync(palPath, "utf-8").split(/\r?\n/);
  const bank: Record<string, Palette> = {};
  let currentLabel: string | null = null;
  let colours: Palette = [];
  for (const raw of lines) {
    const line = raw.trim();
    if (!line) {
      continue;
    }
    if (line.startsWith(";")) {
      const label = sanitizeLabel(line.slice(1).trim());
      if (label) {
        currentLabel = label;
        colours = [];
      }
      continue;
    }
    if (!line.toUpperCase().startsWith("RGB")) {
      continue;
    }
    if (!currentLabel) {
      continue;
    }
    const parts = line.split("RGB", 2)[1].split(",").map((part) => part.trim());
    if (parts.length < 3) {
      throw new Error(`Malformed RGB line '${line}' in ${palPath}`);
    }
    const [r, g, b] = parts.slice(0, 3).map((part) => Number(part));
    colours.push([scaleComponent(r), scaleComponent(g), scaleComponent(b)]);
    if (colours.length === 4) {
      bank[currentLabel] = colours;
      colours = [];
      currentLabel = null;
    }
  }
  if (Object.keys(bank).length === 0) {
    throw new Error(`No palettes parsed from ${palPath}`);
  }
  return bank;
};

const loadTilePaletteMap = (assetsRoot: string): Record<string, string[]> => {
  const jsonPath = joinPath(assetsRoot, "data", POKEGEAR_TOWN_MAP_PALETTE_MAP);
  return readJsonAssetSync<Record<string, string[]>>(jsonPath);
};

const applyPalette = (surface: Surface, palette: Palette): void => {
  for (let y = 0; y < TILE_SIZE; y += 1) {
    for (let x = 0; x < TILE_SIZE; x += 1) {
      const gray = surface.getAt(x, y)[0];
      let idx = 0;
      if (gray >= 192) {
        idx = 3;
      } else if (gray >= 128) {
        idx = 2;
      } else if (gray >= 64) {
        idx = 1;
      }
      const [r, g, b] = palette[idx];
      surface.setAt(x, y, [r, g, b, 255]);
    }
  }
};

const decodeRleStream = (data: Buffer): number[] => {
  const out = Array(TILE_COUNT).fill(0x4f);
  let idx = 0;
  let i = 0;
  while (i < data.length) {
    const tile = data[i];
    i += 1;
    if (tile === 0xff) {
      break;
    }
    if (i >= data.length) {
      throw new Error("Malformed RLE stream: count missing");
    }
    const count = data[i];
    i += 1;
    if (idx + count > out.length) {
      throw new Error("RLE decode overflowed screen tile capacity");
    }
    for (let c = 0; c < count; c += 1) {
      out[idx] = tile & 0xff;
      idx += 1;
    }
  }
  return out;
};

class TileBank {
  constructor(public readonly tiles: Surface[]) {}

  getTile(index: number): Surface {
    if (index >= 0 && index < this.tiles.length) {
      return this.tiles[index];
    }
    const debug = new Surface(TILE_SIZE, TILE_SIZE);
    debug.fill([255, 0, 255, 255]);
    return debug;
  }
}

export class PokegearVRAMSpan {
  constructor(
    public readonly startTile: number,
    public readonly tileCount: number,
    public readonly source: string,
    public readonly target: string,
    public readonly bank: number = 0,
  ) {}

  endTile(): number {
    return this.startTile + this.tileCount - 1;
  }
}

export class PokegearHardwareRegisters {
  public scx = 0;
  public scy = 0;
  public wx = POKEGEAR_INIT_WX;
  public wy = POKEGEAR_INIT_WY;
  public lcdcPointer: number | null = null;

  setScroll(scx: number, scy: number): void {
    this.scx = scx & 0xff;
    this.scy = scy & 0xff;
  }

  setWindow(wx: number, wy: number): void {
    this.wx = wx & 0xff;
    this.wy = wy & 0xff;
  }

  setLcdcPointer(pointer: number | null): void {
    this.lcdcPointer = pointer === null ? null : pointer & 0xffff;
  }
}

export class PokegearHardwareState {
  public registers = new PokegearHardwareRegisters();
  public lcdEnabled = true;
  public vbank1Cleared = false;
  public lcdTransitions: string[] = [];
  public vramSpans: PokegearVRAMSpan[] = [];
  public oamLoads: PokegearVRAMSpan[] = [];

  reset(): void {
    this.registers = new PokegearHardwareRegisters();
    this.lcdEnabled = true;
    this.vbank1Cleared = false;
    this.lcdTransitions = [];
    this.vramSpans = [];
    this.oamLoads = [];
  }

  disableLcd(): void {
    this.lcdEnabled = false;
    this.lcdTransitions.push("disable");
  }

  enableLcd(): void {
    this.lcdEnabled = true;
    this.lcdTransitions.push("enable");
  }

  clearVbank1(): void {
    this.vbank1Cleared = true;
  }

  recordOamLoad(startTile: number, tileCount: number, source: string, bank: number = 0): void {
    this.oamLoads.push(new PokegearVRAMSpan(startTile & 0xff, tileCount, source, "wShadowOAM", bank));
  }

  recordVramLoad(startTile: number, tileCount: number, source: string, target: string, bank: number = 0): void {
    if (tileCount <= 0) {
      throw new Error(`${source} attempted to load non-positive tile count.`);
    }
    const span = new PokegearVRAMSpan(startTile & 0xff, tileCount, source, target, bank);
    this.verifyNoOverlap(span);
    this.vramSpans.push(span);
  }

  private verifyNoOverlap(span: PokegearVRAMSpan): void {
    for (const existing of this.vramSpans) {
      if (existing.target !== span.target || existing.bank !== span.bank) {
        continue;
      }
      if (existing.endTile() < span.startTile) {
        continue;
      }
      if (span.endTile() < existing.startTile) {
        continue;
      }
      throw new Error(
        `VRAM span ${span.startTile.toString(16)}-${span.endTile().toString(16)} from ${span.source} overlaps ` +
          `${existing.startTile.toString(16)}-${existing.endTile().toString(16)} (${existing.source})`,
      );
    }
  }
}

type PokegearBackgroundOptions = {
  playerGender?: PlayerGender | number | null;
  basePath?: string | null;
  mapGroup?: number | null;
  mapNumber?: number | null;
};

export class PokegearBackground {
  public readonly hardware = new PokegearHardwareState();
  private readonly assetsRoot: string;
  private readonly gfxDir: string;
  private readonly playerGfxDir: string;
  private readonly tilePaletteMap: Record<string, string[]>;
  private readonly paletteBanks: Record<PlayerGender, Record<string, Palette>>;
  private readonly baseTownTiles: Surface[];
  private readonly basePgearTiles: Surface[];
  private readonly spriteTileSet: Surface[];
  private readonly fastShipTiles: Surface[];
  private readonly regionTilemaps: Record<string, number[]>;
  private readonly cardTilemaps: Record<number, number[]>;
  private readonly banks = new Map<PlayerGender, TileBank>();
  private readonly fontTileCache = new Map<string, Record<number, Surface>>();
  private readonly playerIconCache = new Map<PlayerGender, Surface[]>();
  private readonly npcPaletteManager = new NpcPaletteManager();
  private frameTileLevels: number[][] | null = null;
  private spaceTileLevels: number[] | null = null;
  private fontTileLevels: number[][] | null = null;
  private gender: PlayerGender;
  private lastHardwareSignature: string | null = null;

  constructor(playerGenderOrOptions?: PlayerGender | number | null | PokegearBackgroundOptions) {
    const options =
      playerGenderOrOptions && typeof playerGenderOrOptions === "object"
        ? (playerGenderOrOptions as PokegearBackgroundOptions)
        : { playerGender: playerGenderOrOptions ?? null };
    const basePath = options.basePath ?? null;
    const assetRoot = basePath
      ? assetExists(joinPath(basePath, "assets"))
        ? joinPath(basePath, "assets")
        : basePath
      : getAssetsRoot();
    this.assetsRoot = assetRoot;
    this.gfxDir = joinPath(assetRoot, "gfx", "pokegear");
    this.playerGfxDir = joinPath(assetRoot, "gfx", "player");
    this.tilePaletteMap = loadTilePaletteMap(assetRoot);
    this.paletteBanks = this.loadPaletteBanks();
    this.baseTownTiles = loadTilesetSurfaces(this.gfxDir, "town_map");
    this.basePgearTiles = loadTilesetSurfaces(this.gfxDir, "pokegear");
    for (const [gender, paletteBank] of Object.entries(this.paletteBanks)) {
      const g = (gender === "MALE" || gender === "0") ? PlayerGender.MALE : PlayerGender.FEMALE;
      this.banks.set(g, this.buildBankForPalette(paletteBank));
    }

    this.cardTilemaps = this.loadCardTilemaps();
    this.regionTilemaps = {
      JOHTO: this.loadRegionMap("johto.bin"),
      KANTO: this.loadRegionMap("kanto.bin"),
    };
    this.spriteTileSet = loadTilesetSurfaces(this.gfxDir, "pokegear_sprites");
    this.fastShipTiles = loadTilesetSurfaces(this.gfxDir, "fast_ship");
    this.gender = normalizeGender(options.playerGender ?? null);
    if (!this.banks.has(this.gender)) {
      this.gender = PlayerGender.MALE;
    }
    this.syncHardware({ mapGroup: options.mapGroup ?? null, mapNumber: options.mapNumber ?? null });
  }

  setPlayerGender(gender: PlayerGender | number | null): void {
    const normalized = normalizeGender(gender);
    this.gender = this.banks.has(normalized) ? normalized : PlayerGender.MALE;
    this.lastHardwareSignature = null;
  }

  tilemapForCard(card: number, region?: string | null): number[] {
    if (card === 1) {
      const key = String(region ?? "JOHTO").toUpperCase() === "KANTO" ? "KANTO" : "JOHTO";
      return [...(this.regionTilemaps[key] ?? this.regionTilemaps.JOHTO)];
    }
    const base = this.cardTilemaps[card];
    if (!base) {
      return Array(TILE_COUNT).fill(0x4f);
    }
    return [...base];
  }

  tileSurfaces(): Surface[] {
    return this.currentBank().tiles;
  }

  spriteTiles(): Surface[] {
    return this.spriteTileSet;
  }

  playerIconSurface(timeOfDay?: string | null): Surface {
    const sourceTiles = this.playerIconTiles(this.gender).slice(0, 4);
    this.verifyPlayerIconLength(this.playerIconTiles(this.gender));
    const paletteId = this.gender === PlayerGender.FEMALE ? 1 : 0;
    const palette = this.npcPaletteManager.palette(paletteId, timeOfDay);
    const icon = new Surface(16, 16);
    sourceTiles.forEach((tile, tileIndex) => {
      const tileX = (tileIndex % 2) * TILE_SIZE;
      const tileY = Math.floor(tileIndex / 2) * TILE_SIZE;
      for (let y = 0; y < TILE_SIZE; y += 1) {
        for (let x = 0; x < TILE_SIZE; x += 1) {
          const level = Math.max(0, Math.min(3, Math.round(tile.getAt(x, y)[0] / 85)));
          const [r, g, b] = palette[level];
          icon.setAt(tileX + x, tileY + y, [r, g, b, level === 0 ? 0 : 255]);
        }
      }
    });
    return icon;
  }

  windowFillColor(): [number, number, number] {
    const palette = this.currentPaletteBank().border;
    if (!palette) {
      return [230, 255, 164];
    }
    return palette[0];
  }

  pointerHighlightColor(): [number, number, number] {
    const palette = this.currentPaletteBank().point_of_interest;
    if (!palette || palette.length < 4) {
      return [248, 72, 80];
    }
    return palette[3];
  }

  drawCardBackground(surface: Surface, card: number): void {
    const tiles = this.tilemapForCard(card);
    this.blitTiles(surface, tiles);
  }

  syncHardware(opts: { mapGroup?: number | null; mapNumber?: number | null }): void {
    const signature = `${opts.mapGroup ?? "none"}:${opts.mapNumber ?? "none"}:${this.gender}`;
    if (signature === this.lastHardwareSignature) {
      return;
    }
    this.lastHardwareSignature = signature;
    this.runLoadSequence(opts.mapGroup ?? null, opts.mapNumber ?? null);
  }

  private currentBank(): TileBank {
    return this.banks.get(this.gender) ?? this.banks.get(PlayerGender.MALE)!;
  }

  private currentPaletteBank(): Record<string, Palette> {
    return this.paletteBanks[this.gender] ?? this.paletteBanks[PlayerGender.MALE];
  }

  private loadPaletteBanks(): Record<PlayerGender, Record<string, Palette>> {
    const male = parsePaletteBank(joinPath(this.gfxDir, POKEGEAR_PAL));
    let female = male;
    const femalePath = joinPath(this.gfxDir, POKEGEAR_F_PAL);
    if (assetExists(femalePath)) {
      female = parsePaletteBank(femalePath);
    }
    return {
      [PlayerGender.MALE]: male,
      [PlayerGender.FEMALE]: female,
    };
  }

  private buildBankForPalette(paletteBank: Record<string, Palette>): TileBank {
    const townTiles = this.copyTiles(this.baseTownTiles);
    const pgearTiles = this.copyTiles(this.basePgearTiles);
    this.applyTilePalettes(townTiles, this.tilePaletteMap.town_map ?? [], paletteBank);
    this.applyTilePalettes(pgearTiles, this.tilePaletteMap.pokegear ?? [], paletteBank);
    const bank = this.composeBank(townTiles, pgearTiles);
    this.injectFontTiles(bank, paletteBank);
    return bank;
  }

  private copyTiles(tiles: Surface[]): Surface[] {
    return tiles.map((tile) => tile.copy());
  }

  private composeBank(townTiles: Surface[], pgearTiles: Surface[]): TileBank {
    const maxLen = Math.max(0x30 + pgearTiles.length, townTiles.length);
    const tiles: Surface[] = Array.from({ length: maxLen }, () => new Surface(TILE_SIZE, TILE_SIZE));
    townTiles.forEach((tile, index) => {
      if (index < tiles.length) {
        tiles[index] = tile;
      }
    });
    pgearTiles.forEach((tile, index) => {
      const slot = 0x30 + index;
      while (tiles.length <= slot) {
        tiles.push(new Surface(TILE_SIZE, TILE_SIZE));
      }
      tiles[slot] = tile;
    });
    return new TileBank(tiles);
  }

  private ensureBankCapacity(bank: TileBank, size: number): void {
    if (bank.tiles.length >= size) {
      return;
    }
    while (bank.tiles.length < size) {
      const blank = new Surface(TILE_SIZE, TILE_SIZE);
      blank.fill([0, 0, 0, 0]);
      bank.tiles.push(blank);
    }
  }

  private fontTilesForPalette(paletteBank: Record<string, Palette>): Record<number, Surface> {
    const palette = paletteBank.border;
    if (!palette) {
      throw new Error("Pokegear palette bank missing 'border' entry");
    }
    const key = palette.map((entry) => entry.join(",")).join("|");
    const cached = this.fontTileCache.get(key);
    if (cached) {
      return cached;
    }
    if (!this.frameTileLevels) {
      this.frameTileLevels = load1bppTileLevels(joinPath(this.assetsRoot, "gfx", "frames"), "1");
    }
    if (this.frameTileLevels.length < 6) {
      throw new Error("Frame tileset must contain at least 6 tiles.");
    }
    if (!this.spaceTileLevels) {
      const levels = load2bppTileLevels(joinPath(this.assetsRoot, "gfx", "font"), "space");
      if (!levels.length) {
        throw new Error("Space tile could not be loaded.");
      }
      this.spaceTileLevels = levels[0];
    }
    if (!this.fontTileLevels) {
      this.fontTileLevels = load1bppTileLevels(joinPath(this.assetsRoot, "gfx", "font"), "font");
    }
    if (this.fontTileLevels.length < 0x80) {
      throw new Error("Font tileset must contain at least 128 tiles.");
    }
    const tiles: Record<number, Surface> = {};
    const frameTileIds = [0x79, 0x7a, 0x7b, 0x7c, 0x7d, 0x7e];
    frameTileIds.forEach((tileId, index) => {
      tiles[tileId] = renderTileFromLevels(this.frameTileLevels![index], palette);
    });
    tiles[0x7f] = renderTileFromLevels(this.spaceTileLevels!, palette);
    this.fontTileLevels.slice(0, 0x80).forEach((levels, index) => {
      tiles[0x80 + index] = renderTileFromLevels(levels, palette);
    });
    this.fontTileCache.set(key, tiles);
    return tiles;
  }

  private injectFontTiles(bank: TileBank, paletteBank: Record<string, Palette>): void {
    const fontTiles = this.fontTilesForPalette(paletteBank);
    this.ensureBankCapacity(bank, 0x100);
    for (const [id, tile] of Object.entries(fontTiles)) {
      const tileId = Number(id);
      if (!Number.isNaN(tileId)) {
        bank.tiles[tileId] = tile;
      }
    }
  }

  private loadCardTilemaps(): Record<number, number[]> {
    const tilemaps: Record<number, number[]> = {};
    const cardNames: Record<number, string> = {
      0: "clock",
      2: "phone",
      3: "radio",
    };
    for (const [card, name] of Object.entries(cardNames)) {
      const pathName = joinPath(this.gfxDir, `${name}.tilemap.rle`);
      if (!assetExists(pathName)) {
        continue;
      }
      const decoded = decodeRleStream(readBytes(pathName));
      tilemaps[Number(card)] = this.applyTextbox(decoded);
    }
    return tilemaps;
  }

  private applyTextbox(tiles: number[]): number[] {
    const tilemap = new TilemapSurface();
    tilemap.loadTiles(tiles);
    tilemap.drawWindow(0, 12, 20, 6);
    const [flattened] = tilemap.flatten();
    return flattened;
  }

  private applyTilePalettes(tiles: Surface[], paletteTokens: string[], paletteBank: Record<string, Palette>): void {
    if (!paletteTokens.length) {
      throw new Error("Tile palette tokens missing for pokegear assets");
    }
    if (paletteTokens.length < tiles.length) {
      throw new Error(`Palette token count ${paletteTokens.length} does not match tile count ${tiles.length}`);
    }
    const tokens = paletteTokens.slice(0, tiles.length);
    if (paletteTokens.length > tiles.length) {
      const trailing = paletteTokens.slice(tiles.length);
      if (trailing.some((token) => token.toUpperCase() !== "BORDER")) {
        throw new Error(`Unexpected palette tokens beyond tile data boundary: ${trailing.join(", ")}`);
      }
    }
    tiles.forEach((tile, idx) => {
      const token = tokens[idx].toUpperCase();
      const paletteName = TOKEN_TO_LABEL[token];
      if (!paletteName) {
        throw new Error(`Unknown palette token ${token}`);
      }
      const palette = paletteBank[paletteName];
      if (!palette) {
        throw new Error(`Palette '${paletteName}' missing from pokegear.pal`);
      }
      applyPalette(tile, palette);
    });
  }

  private loadRegionMap(filename: string): number[] {
    const filePath = joinPath(this.gfxDir, filename);
    let data = Array.from(readBytes(filePath));
    if (!data.length) {
      throw new Error(`Missing region map: ${filePath}`);
    }
    if (data[data.length - 1] === 0xff) {
      data = data.slice(0, -1);
    }
    if (data.length !== TILE_COUNT) {
      throw new Error(`${filename} contains ${data.length} tiles, expected ${TILE_COUNT}`);
    }
    return data.map((byte) => byte & 0xff);
  }

  private blitTiles(surface: Surface, tiles: number[]): void {
    const bank = this.currentBank();
    let idx = 0;
    for (let ty = 0; ty < SCREEN_TILES_H; ty += 1) {
      for (let tx = 0; tx < SCREEN_TILES_W; tx += 1) {
        const tileId = tiles[idx];
        idx += 1;
        surface.blit(bank.getTile(tileId), [tx * TILE_SIZE, ty * TILE_SIZE]);
      }
    }
  }

  private runLoadSequence(mapGroup: number | null, mapNumber: number | null): void {
    this.hardware.reset();
    this.hardware.disableLcd();
    this.hardware.clearVbank1();
    this.hardware.registers.setScroll(0, 0);
    this.hardware.registers.setWindow(POKEGEAR_INIT_WX, POKEGEAR_INIT_WY);
    this.hardware.recordVramLoad(0, this.baseTownTiles.length, "TownMapGFX", "vTiles2");
    this.hardware.recordVramLoad(0x30, this.basePgearTiles.length, "PokegearGFX", "vTiles2");
    this.hardware.recordVramLoad(0, this.spriteTileSet.length, "PokegearSpritesGFX", "vTiles0");
    this.loadPlayerIcon(mapGroup, mapNumber);
    this.hardware.enableLcd();
  }

  private loadPlayerIcon(mapGroup: number | null, mapNumber: number | null): void {
    const location = this.resolveWorldLocation(mapGroup, mapNumber);
    if (location === LANDMARK_FAST_SHIP) {
      this.recordFastShipIcon();
      return;
    }
    const tiles = this.playerIconTiles(this.gender);
    this.verifyPlayerIconLength(tiles);
    this.hardware.recordVramLoad(0x10, 4, "PlayerIconStanding", "vTiles0");
    this.hardware.recordVramLoad(0x14, 4, "PlayerIconWalking", "vTiles0");
    this.hardware.recordOamLoad(0x10, 4, "PlayerIconStanding");
    this.hardware.recordOamLoad(0x14, 4, "PlayerIconWalking");
  }

  private recordFastShipIcon(): void {
    if (this.fastShipTiles.length < 8) {
      throw new Error("Fast ship icon requires 8 tiles.");
    }
    this.hardware.recordVramLoad(0x10, 8, "FastShipGFX", "vTiles0");
    this.hardware.recordOamLoad(0x10, 8, "FastShipGFX");
  }

  private playerIconTiles(gender: PlayerGender): Surface[] {
    const cached = this.playerIconCache.get(gender);
    if (cached) {
      return cached;
    }
    const stem = gender === PlayerGender.FEMALE ? "kris" : "chris";
    const tiles = loadTilesetSurfaces(this.playerGfxDir, stem);
    this.playerIconCache.set(gender, tiles);
    return tiles;
  }

  private verifyPlayerIconLength(tiles: Surface[]): void {
    if (tiles.length < 0x10) {
      throw new Error(`Player icon sheet must contain at least 16 tiles, found ${tiles.length}`);
    }
  }

  private resolveWorldLocation(mapGroup: number | null, mapNumber: number | null): number | null {
    if (mapGroup === null || mapNumber === null) {
      return null;
    }
    return getWorldMapLocation(mapGroup, mapNumber);
  }
}
