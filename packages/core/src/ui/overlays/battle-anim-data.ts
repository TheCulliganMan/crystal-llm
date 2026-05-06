import fs from 'fs';
import path from 'path';
import { readJsonAssetSync, readTextAssetSync } from '@pokecrystal/core/core/asset-reader';
import { gbc5To8 } from '@pokecrystal/core/core/gbc-colors';
import { decompress } from '@pokecrystal/core/core/lz';
import { getAssetPath, getAssetsRoot, getDataDir } from '@pokecrystal/core/core/paths';
import { gameEngine } from '../game-engine';

// ASM mapping: pokecrystal_disassembly/data/battle_anims/*.asm + gfx/battle_anims.asm.

const INT_RE = /^([-+]?(?:0x[0-9a-fA-F]+|\$[0-9a-fA-F]+|0b[01_]+|%[01_]+|\d+))$/;
const FLAG_SYMBOLS: Record<string, number> = {
  ABSOLUTE_X: 0x00,
  RELATIVE_X: 0x01,
  OAM_XFLIP: 0x20,
  OAM_YFLIP: 0x40,
  OAM_PRIO: 0x80,
};
const OBJECT_ALIASES: Record<string, string> = {
  BATTLE_ANIM_OBJ_THUNDERSHOCK_BALL: 'BATTLE_ANIM_OBJ_THUNDERSHOCK_CORE',
  BATTLE_ANIM_OBJ_SPARKS_CIRCLE: 'BATTLE_ANIM_OBJ_THUNDERSHOCK_SPARKS',
  BATTLE_ANIM_OBJ_THUNDERBOLT_BALL: 'BATTLE_ANIM_OBJ_THUNDERSHOCK_CORE',
  BATTLE_ANIM_OBJ_SPARKS_CIRCLE_BIG: 'BATTLE_ANIM_OBJ_THUNDERSHOCK_SPARKS',
  BATTLE_ANIM_OBJ_SKULL: 'BATTLE_ANIM_OBJ_SKULL_CROSSBONE',
};

const BATTLE_ANIM_BUNDLE_JSON_PATH = path.join(getDataDir(), 'battle_anim_bundle.json');

const require_battle_anim_source = (targetPath: string, label: string): string => {
  try {
    readTextAssetSync(targetPath);
  } catch {
    throw new Error(
      `Battle animation ${label} is required for bundled runtime: missing ${targetPath}`
    );
  }
  return targetPath;
};

const missing_battle_anim_bundle_error = (targetPath: string): Error =>
  new Error(
    `Battle animation runtime bundle is required for bundled runtime: missing or invalid ${targetPath}`
  );

const parse_int_token = (token: string): number => {
  const trimmed = token.trim();
  const match = INT_RE.exec(trimmed);
  if (!match) {
    throw new Error(`Cannot parse integer token: ${token}`);
  }
  const text = match[1];
  const prefix = text.slice(0, 2).toLowerCase();
  if (text.startsWith('$')) {
    return Number.parseInt(text.slice(1), 16);
  }
  if (text.startsWith('%')) {
    return Number.parseInt(text.slice(1).replace(/_/g, ''), 2);
  }
  if (prefix === '0x') {
    return Number.parseInt(text, 16);
  }
  if (prefix === '0b') {
    return Number.parseInt(text.slice(2).replace(/_/g, ''), 2);
  }
  return Number.parseInt(text, 10);
};

const normalize_identifier = (value: string): string => value.trim().toUpperCase();

const battle_anim_asset_candidates = (assetPath: string): string[] => {
  const candidates = [assetPath];
  if (assetPath.endsWith('.2bpp.lz')) {
    candidates.push(assetPath.replace(/\.2bpp\.lz$/, '.2bpp'));
  } else if (assetPath.endsWith('.1bpp.lz')) {
    candidates.push(assetPath.replace(/\.1bpp\.lz$/, '.1bpp'));
  }
  return Array.from(new Set(candidates));
};

const preferred_battle_anim_asset_path = (assetPath: string): string => {
  for (const candidate of battle_anim_asset_candidates(assetPath)) {
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }
  return assetPath;
};

const fallback_battle_anim_asset_path = (assetPath: string): string | null => {
  const [, ...alternates] = battle_anim_asset_candidates(assetPath);
  return alternates[0] ?? null;
};

const canonical_browser_battle_anim_asset_path = (assetPath: string): string => {
  const normalized = assetPath.replace(/\\/g, '/');
  const browserRelative = normalized.includes('/assets/')
    ? normalized.slice(normalized.lastIndexOf('/assets/') + '/assets/'.length)
    : normalized.replace(/^\/+/, '');
  const canonicalRelative = browserRelative
    .replace(/\.2bpp\.lz$/, '.2bpp')
    .replace(/\.1bpp\.lz$/, '.1bpp');
  return getAssetPath(...canonicalRelative.split('/').filter(Boolean));
};

const resolve_battle_anim_asset_path = (assetPath: string): string => {
  if (typeof window !== 'undefined') {
    return canonical_browser_battle_anim_asset_path(assetPath);
  }
  return preferred_battle_anim_asset_path(assetPath);
};

const parse_flag_expression = (raw: string): number => {
  const parts = raw.split('|').map((part) => part.trim());
  let value = 0;
  for (const part of parts) {
    if (part in FLAG_SYMBOLS) {
      value |= FLAG_SYMBOLS[part];
      continue;
    }
    value |= parse_int_token(part);
  }
  return value;
};

const parse_nonnegative_int = (value: unknown): number => {
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) {
      throw new Error(`Cannot parse integer token: ${String(value)}`);
    }
    return Math.max(0, Math.trunc(value));
  }
  return Math.max(0, parse_int_token(String(value ?? '0')));
};

export interface BattleAnimObjectDef {
  object_id: string;
  flags: number;
  fix_y: number;
  function: string | null;
  frameset: string;
  palette: string;
  gfx_id: string;
}

export interface BattleAnimFrame {
  command: 'frame' | 'wait' | 'end' | 'restart' | 'delete';
  oam_set: string | null;
  duration: number;
  xflip: boolean;
  yflip: boolean;
}

export interface OamEntry {
  x: number;
  y: number;
  tile_id: number;
  xflip: boolean;
  yflip: boolean;
}

export interface BattleAnimOAMSet {
  name: string;
  tile_offset: number;
  entries: OamEntry[];
}

interface TileSheet {
  gfx_id: string;
  tiles: number[][][];
}

interface BattleAnimRuntimeBundle {
  objects: Record<string, BattleAnimObjectDef>;
  framesets: Record<string, BattleAnimFrame[]>;
  oam_sets: Record<string, BattleAnimOAMSet>;
  gfx_table: Record<string, [number, string]>;
  gfx_sources: Record<string, string>;
}

type SurfaceLike = {
  get_size: () => [number, number];
  get_at: (position: [number, number]) => [number, number, number, number];
};

export interface RenderedSprite {
  surface: InstanceType<typeof gameEngine.Surface>;
  offset_x: number;
  offset_y: number;
}

export class BattleAnimData {
  private readonly assetsRoot: string;
  private readonly bundle: BattleAnimRuntimeBundle;
  private readonly objects: Map<string, BattleAnimObjectDef>;
  private readonly framesets: Map<string, BattleAnimFrame[]>;
  private readonly oamSets: Map<string, BattleAnimOAMSet>;
  private readonly gfxTable: Map<string, [number, string]>;
  private readonly gfxSources: Map<string, string>;
  private readonly tileCache: Map<string, InstanceType<typeof gameEngine.Surface>> = new Map();
  private readonly tileSheets: Map<string, TileSheet> = new Map();
  private readonly renderCache: Map<string, RenderedSprite> = new Map();
  private readonly palettes: Map<string, Array<[number, number, number, number]>>;
  public readonly object_defs: Map<string, BattleAnimObjectDef>;

  constructor() {
    this.assetsRoot = getAssetsRoot();
    this.bundle = this.load_runtime_bundle();
    this.objects = this.load_object_defs();
    this.apply_object_aliases();
    this.framesets = this.load_framesets();
    this.oamSets = this.load_oam_sets();
    this.gfxTable = this.load_gfx_table();
    this.gfxSources = this.parse_gfx_sources();
    this.palettes = this.load_palettes();
    this.object_defs = this.objects;
  }

  private load_runtime_bundle(): BattleAnimRuntimeBundle {
    let parsed: unknown;
    try {
      parsed = readJsonAssetSync<unknown>(BATTLE_ANIM_BUNDLE_JSON_PATH);
    } catch {
      throw missing_battle_anim_bundle_error(BATTLE_ANIM_BUNDLE_JSON_PATH);
    }
    if (!parsed || typeof parsed !== 'object') {
      throw missing_battle_anim_bundle_error(BATTLE_ANIM_BUNDLE_JSON_PATH);
    }
    const record = parsed as Partial<BattleAnimRuntimeBundle>;
    if (
      !record.objects || typeof record.objects !== 'object' ||
      !record.framesets || typeof record.framesets !== 'object' ||
      !record.oam_sets || typeof record.oam_sets !== 'object' ||
      !record.gfx_table || typeof record.gfx_table !== 'object' ||
      !record.gfx_sources || typeof record.gfx_sources !== 'object'
    ) {
      throw missing_battle_anim_bundle_error(BATTLE_ANIM_BUNDLE_JSON_PATH);
    }
    return {
      objects: record.objects as Record<string, BattleAnimObjectDef>,
      framesets: record.framesets as Record<string, BattleAnimFrame[]>,
      oam_sets: record.oam_sets as Record<string, BattleAnimOAMSet>,
      gfx_table: record.gfx_table as Record<string, [number, string]>,
      gfx_sources: record.gfx_sources as Record<string, string>,
    };
  }

  private load_object_defs(): Map<string, BattleAnimObjectDef> {
    const objects = new Map<string, BattleAnimObjectDef>();
    for (const [objectId, value] of Object.entries(this.bundle.objects)) {
      objects.set(normalize_identifier(objectId), {
        ...value,
        object_id: normalize_identifier(value.object_id ?? objectId),
        function: value.function ? normalize_identifier(value.function) : null,
        frameset: normalize_identifier(value.frameset),
        palette: normalize_identifier(value.palette),
        gfx_id: normalize_identifier(value.gfx_id),
      });
    }
    return objects;
  }

  private apply_object_aliases(): void {
    for (const [alias, target] of Object.entries(OBJECT_ALIASES)) {
      if (this.objects.has(alias)) {
        continue;
      }
      const targetDef = this.objects.get(target);
      if (!targetDef) {
        continue;
      }
      this.objects.set(alias, targetDef);
    }
  }

  private load_gfx_table(): Map<string, [number, string]> {
    const table = new Map<string, [number, string]>();
    for (const [name, entry] of Object.entries(this.bundle.gfx_table)) {
      if (!Array.isArray(entry) || entry.length < 2) {
        continue;
      }
      table.set(normalize_identifier(name), [Number(entry[0]), String(entry[1]).trim()]);
    }
    return table;
  }

  private load_framesets(): Map<string, BattleAnimFrame[]> {
    const framesets = new Map<string, BattleAnimFrame[]>();
    for (const [framesetName, entries] of Object.entries(this.bundle.framesets)) {
      framesets.set(
        normalize_identifier(framesetName),
        (entries ?? []).map((entry) => ({
          command: entry.command,
          oam_set: entry.oam_set ? normalize_identifier(entry.oam_set) : null,
          duration: parse_nonnegative_int(entry.duration ?? 0),
          xflip: Boolean(entry.xflip),
          yflip: Boolean(entry.yflip),
        })),
      );
    }
    return framesets;
  }

  public resolve_frameset_name(objectId: string, framesetOverride?: string | null): string | null {
    if (!objectId) {
      return null;
    }
    const normalizedId = normalize_identifier(objectId);
    const obj = this.objects.get(normalizedId);
    if (!obj) {
      return null;
    }
    return framesetOverride ?? obj.frameset;
  }

  public get_frameset_frames(name: string): BattleAnimFrame[] | null {
    if (!name) {
      return null;
    }
    return this.framesets.get(normalize_identifier(name)) ?? null;
  }

  public get_oam_set(name: string): BattleAnimOAMSet | null {
    if (!name) {
      return null;
    }
    return this.oamSets.get(normalize_identifier(name)) ?? null;
  }

  private load_oam_sets(): Map<string, BattleAnimOAMSet> {
    const sets = new Map<string, BattleAnimOAMSet>();
    for (const [oamName, value] of Object.entries(this.bundle.oam_sets)) {
      sets.set(normalize_identifier(oamName), {
        name: normalize_identifier(value.name ?? oamName),
        tile_offset: Number(value.tile_offset ?? 0),
        entries: (value.entries ?? []).map((entry) => ({
          x: Number(entry.x ?? 0),
          y: Number(entry.y ?? 0),
          tile_id: Number(entry.tile_id ?? 0),
          xflip: Boolean(entry.xflip),
          yflip: Boolean(entry.yflip),
        })),
      });
    }
    return sets;
  }

  private load_palettes(): Map<string, Array<[number, number, number, number]>> {
    const palettePath = require_battle_anim_source(
      getAssetPath('gfx', 'battle_anims', 'battle_anims.pal'),
      'palettes'
    );
    const palettes = new Map<string, Array<[number, number, number, number]>>();
    let name: string | null = null;
    const lines = readTextAssetSync(palettePath).split(/\r?\n/);
    for (const raw of lines) {
      const stripped = raw.trim();
      if (!stripped) {
        continue;
      }
      if (stripped.startsWith(';')) {
        name = stripped.replace(/^;+/g, '').trim().toLowerCase();
        continue;
      }
      if (!stripped.startsWith('RGB') || !name) {
        continue;
      }
      const rgbTokens = stripped
        .slice('RGB'.length)
        .split(',')
        .map((token) => token.trim())
        .filter(Boolean);
      if (rgbTokens.length !== 3) {
        continue;
      }
      const r = Number.parseInt(rgbTokens[0], 10);
      const g = Number.parseInt(rgbTokens[1], 10);
      const b = Number.parseInt(rgbTokens[2], 10);
      if ([r, g, b].some((value) => Number.isNaN(value))) {
        continue;
      }
      const rgba: [number, number, number, number] = [
        gbc5To8(r),
        gbc5To8(g),
        gbc5To8(b),
        255,
      ];
      if (!palettes.has(name)) {
        palettes.set(name, []);
      }
      palettes.get(name)?.push(rgba);
    }

    for (const values of palettes.values()) {
      if (values.length) {
        const [r, g, b] = values[0];
        values[0] = [r, g, b, 0];
      }
    }
    return palettes;
  }

  private parse_gfx_sources(): Map<string, string> {
    const mapping = new Map<string, string>();
    for (const [label, relPath] of Object.entries(this.bundle.gfx_sources)) {
      const preferred = path.isAbsolute(relPath)
        ? relPath
        : path.resolve(this.assetsRoot, relPath);
      const resolved = resolve_battle_anim_asset_path(preferred);
      mapping.set(label.trim(), resolved);
    }
    return mapping;
  }

  private decode_battle_anim_tile_sheet(
    assetPath: string,
  ): { data: Buffer; resolvedPath: string } | null {
    if (!fs.existsSync(assetPath)) {
      return null;
    }
    const raw = fs.readFileSync(assetPath);
    const data = assetPath.endsWith('.lz') ? Buffer.from(decompress(raw)) : Buffer.from(raw);
    if (!data.length || data.length % 16 !== 0) {
      return null;
    }
    return { data, resolvedPath: assetPath };
  }

  private load_tile_sheet(gfxLabel: string): TileSheet | null {
    if (this.tileSheets.has(gfxLabel)) {
      return this.tileSheets.get(gfxLabel) as TileSheet;
    }
    const assetPath = this.gfxSources.get(gfxLabel);
    if (!assetPath) {
      return null;
    }
    const retryPath = fallback_battle_anim_asset_path(assetPath);
    let decoded: { data: Buffer; resolvedPath: string } | null = null;
    for (const candidate of [assetPath, retryPath]) {
      if (!candidate) {
        continue;
      }
      try {
        decoded = this.decode_battle_anim_tile_sheet(candidate);
      } catch {
        decoded = null;
      }
      if (decoded) {
        if (decoded.resolvedPath !== assetPath) {
          this.gfxSources.set(gfxLabel, decoded.resolvedPath);
        }
        break;
      }
    }
    if (!decoded) {
      return null;
    }
    const { data } = decoded;
    const tiles: number[][][] = [];
    for (let base = 0; base + 16 <= data.length; base += 16) {
      const tile: number[][] = Array.from({ length: 8 }, () => Array(8).fill(0));
      for (let row = 0; row < 8; row += 1) {
        const plane0 = data[base + row * 2];
        const plane1 = data[base + row * 2 + 1];
        for (let col = 0; col < 8; col += 1) {
          const bit = 7 - col;
          const colour = ((plane0 >> bit) & 1) | (((plane1 >> bit) & 1) << 1);
          tile[row][col] = colour;
        }
      }
      tiles.push(tile);
    }
    const sheet = { gfx_id: gfxLabel, tiles };
    this.tileSheets.set(gfxLabel, sheet);
    return sheet;
  }

  private tile_surface(
    gfxLabel: string,
    paletteName: string,
    tileIndex: number,
  ): InstanceType<typeof gameEngine.Surface> | null {
    const key = `${gfxLabel}:${paletteName}:${tileIndex}`;
    const cached = this.tileCache.get(key);
    if (cached) {
      return cached;
    }
    const sheet = this.load_tile_sheet(gfxLabel);
    if (!sheet || tileIndex < 0 || tileIndex >= sheet.tiles.length) {
      return null;
    }
    const palette = this.palettes.get(paletteName.toLowerCase());
    if (!palette || !palette.length) {
      throw new Error(`Missing battle animation palette '${paletteName}'.`);
    }
    const surface = new gameEngine.Surface(8, 8);
    const tile = sheet.tiles[tileIndex];
    for (let y = 0; y < 8; y += 1) {
      for (let x = 0; x < 8; x += 1) {
        const colourIndex = tile[y][x];
        const color = palette[colourIndex] ?? palette[0];
        if (colourIndex === 0) {
          surface.set_at([x, y], [color[0], color[1], color[2], 0]);
          continue;
        }
        surface.set_at([x, y], color);
      }
    }
    this.tileCache.set(key, surface);
    return surface;
  }

  private clear_dynamic_gfx_caches(): void {
    this.tileCache.clear();
    this.renderCache.clear();
  }

  private surface_region_to_tiles(
    surface: SurfaceLike,
    originTileX: number,
    originTileY: number,
    widthTiles: number,
    heightTiles: number,
  ): number[][][] {
    const [surfaceWidth, surfaceHeight] = surface.get_size();
    const tileSize = 8;
    if (surfaceWidth % tileSize !== 0 || surfaceHeight % tileSize !== 0) {
      throw new Error(
        `Dynamic battle animation surface must align to ${tileSize}x${tileSize} tiles, got ${surfaceWidth}x${surfaceHeight}.`
      );
    }
    const tiles: number[][][] = [];
    for (let tileY = 0; tileY < heightTiles; tileY += 1) {
      for (let tileX = 0; tileX < widthTiles; tileX += 1) {
        const tile: number[][] = Array.from({ length: tileSize }, () => Array(tileSize).fill(0));
        for (let pixelY = 0; pixelY < tileSize; pixelY += 1) {
          for (let pixelX = 0; pixelX < tileSize; pixelX += 1) {
            const sourceX = (originTileX + tileX) * tileSize + pixelX;
            const sourceY = (originTileY + tileY) * tileSize + pixelY;
            const [r, g, b, a] = surface.get_at([sourceX, sourceY]);
            if (a === 0) {
              tile[pixelY][pixelX] = 0;
              continue;
            }
            const value = Math.floor((r + g + b) / 3);
            tile[pixelY][pixelX] = Math.min(3, Math.max(0, Math.round((255 - value) / 85)));
          }
        }
        tiles.push(tile);
      }
    }
    return tiles;
  }

  register_battler_surfaces(input: {
    playerSurface?: SurfaceLike | null;
    enemySurface?: SurfaceLike | null;
  }): void {
    const { playerSurface, enemySurface } = input;
    if (playerSurface) {
      const [width, height] = playerSurface.get_size();
      const widthTiles = Math.floor(width / 8);
      const heightTiles = Math.floor(height / 8);
      this.tileSheets.set('BATTLE_ANIM_GFX_ENEMYFEET', {
        gfx_id: 'BATTLE_ANIM_GFX_ENEMYFEET',
        tiles: this.surface_region_to_tiles(playerSurface, 0, 0, widthTiles, Math.min(2, heightTiles)),
      });
    }
    if (enemySurface) {
      const [width, height] = enemySurface.get_size();
      const widthTiles = Math.floor(width / 8);
      const heightTiles = Math.floor(height / 8);
      this.tileSheets.set('BATTLE_ANIM_GFX_PLAYERHEAD', {
        gfx_id: 'BATTLE_ANIM_GFX_PLAYERHEAD',
        tiles: this.surface_region_to_tiles(
          enemySurface,
          0,
          Math.max(0, heightTiles - 2),
          widthTiles,
          Math.min(2, heightTiles),
        ),
      });
    }
    this.clear_dynamic_gfx_caches();
  }

  private resolve_palette_override(value: string | null | undefined): string | null {
    if (!value) {
      return null;
    }
    const trimmed = value.trim();
    if (!trimmed) {
      return null;
    }
    const normalized = trimmed.toLowerCase();
    if (this.palettes.has(normalized)) {
      return normalized;
    }
    return this.palette_name(trimmed);
  }

  render_sprite(
    objectId: string,
    frameIndex = 0,
    options?: {
      frameset_override?: string | null;
      palette_override?: string | null;
      extra_xflip?: boolean;
      extra_yflip?: boolean;
    },
  ): RenderedSprite | null {
    if (!objectId) {
      return null;
    }
    const normalizedId = normalize_identifier(objectId);
    const paletteOverride = this.resolve_palette_override(options?.palette_override ?? null);
    const cacheKey = `${normalizedId}:${options?.frameset_override ?? ''}:${paletteOverride ?? ''}:${frameIndex}:${options?.extra_xflip ?? false}:${options?.extra_yflip ?? false}`;
    const cached = this.renderCache.get(cacheKey);
    if (cached) {
      return cached;
    }

    const obj = this.objects.get(normalizedId);
    if (!obj) {
      return null;
    }
    const framesetName = options?.frameset_override ?? obj.frameset;
    const frames = this.framesets.get(framesetName);
    if (!frames || !frames.length) {
      return null;
    }
    if (frameIndex < 0) {
      return null;
    }
    const frame = frames[Math.min(frameIndex, frames.length - 1)];
    if (frame.command !== 'frame' || !frame.oam_set) {
      return null;
    }
    const oamSet = this.oamSets.get(frame.oam_set);
    if (!oamSet) {
      return null;
    }
    const paletteName = paletteOverride ?? this.palette_name(obj.palette);
    const extraXflip = Boolean(options?.extra_xflip);
    const extraYflip = Boolean(options?.extra_yflip);
    const frameXflip = frame.xflip !== extraXflip;
    const frameYflip = frame.yflip !== extraYflip;
    const oamBaseOffset = frames.reduce((min, entry) => {
      if (entry.command !== 'frame' || !entry.oam_set) {
        return min;
      }
      const candidate = this.oamSets.get(entry.oam_set)?.tile_offset;
      if (candidate === undefined) {
        return min;
      }
      return Math.min(min, candidate);
    }, Number.POSITIVE_INFINITY);
    const resolvedBaseOffset = Number.isFinite(oamBaseOffset) ? oamBaseOffset : 0;

    const pieces: Array<{ surface: InstanceType<typeof gameEngine.Surface>; x: number; y: number }> = [];
    let minX = 0;
    let minY = 0;
    let maxX = 0;
    let maxY = 0;

    for (const entry of oamSet.entries) {
      const effectiveXflip = entry.xflip !== frameXflip;
      const effectiveYflip = entry.yflip !== frameYflip;
      const tileIndex = oamSet.tile_offset + entry.tile_id - resolvedBaseOffset;
      const sheetInfo = this.gfxTable.get(obj.gfx_id);
      if (!sheetInfo) {
        throw new Error(
          `Missing battle animation gfx table entry '${obj.gfx_id}' for ${normalizedId}.`
        );
      }
      let gfxLabel = sheetInfo[1];
      if (
        gfxLabel === 'NULL' &&
        ['BATTLE_ANIM_GFX_PLAYERHEAD', 'BATTLE_ANIM_GFX_ENEMYFEET'].includes(obj.gfx_id)
      ) {
        gfxLabel = obj.gfx_id;
      }
      const tile = this.tile_surface(gfxLabel, paletteName, tileIndex);
      if (!tile) {
        throw new Error(
          `Missing battle animation tile ${tileIndex} for ${normalizedId} (${gfxLabel}/${frame.oam_set}).`
        );
      }
      let surface = tile;
      if (effectiveXflip || effectiveYflip) {
        surface = gameEngine.transform.flip(surface, effectiveXflip, effectiveYflip);
      }
      // ASM: BattleAnimOAMUpdate flips frame/object coords with -(coord + 8) when X/Y flip is set.
      const x = frameXflip ? -(entry.x + 8) : entry.x;
      const y = frameYflip ? -(entry.y + 8) : entry.y;
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
      maxX = Math.max(maxX, x + surface.get_width());
      maxY = Math.max(maxY, y + surface.get_height());
      pieces.push({ surface, x, y });
    }

    if (!pieces.length) {
      throw new Error(`Battle animation sprite ${normalizedId} frame ${frameIndex} rendered no OAM pieces.`);
    }

    const width = Math.max(1, maxX - minX);
    const height = Math.max(1, maxY - minY);
    const composite = new gameEngine.Surface(width, height);
    for (const piece of pieces) {
      composite.blit(piece.surface, [piece.x - minX, piece.y - minY]);
    }

    const rendered = { surface: composite, offset_x: minX, offset_y: minY };
    this.renderCache.set(cacheKey, rendered);
    return rendered;
  }

  private palette_name(constant: string): string {
    const mapping: Record<string, string> = {
      PAL_BATTLE_OB_GRAY: 'gray',
      PAL_BATTLE_OB_YELLOW: 'yellow',
      PAL_BATTLE_OB_RED: 'red',
      PAL_BATTLE_OB_GREEN: 'green',
      PAL_BATTLE_OB_BLUE: 'blue',
      PAL_BATTLE_OB_BROWN: 'brown',
      PAL_BATTLE_OB_ENEMY: 'gray',
      PAL_BATTLE_OB_PLAYER: 'brown',
    };
    const normalized = normalize_identifier(constant);
    const palette = mapping[normalized];
    if (!palette) {
      throw new Error(`Unknown battle animation palette constant '${constant}'.`);
    }
    return palette;
  }
}
