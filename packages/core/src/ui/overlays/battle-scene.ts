// ASM: engine/battle/asm (tilemap + scroll wrap used for background scenes).
import { Rect, Surface } from '../surface';
import { BattleBackgroundTilemap, PAL_MENU, PAL_TEXT_WINDOW, build_battle_tilemap, build_battle_tileset } from './_battle-background';
import { BattleUILayoutFactory } from './_battle-layout';
import { BitmapFont } from '../text/bitmap-font';
import { gameEngine } from '../game-engine';

type SceneCacheEntry = {
  revision: number;
  tilesetKey: string;
  sizeKey: string;
  surface: Surface;
};

type WrappedSceneCacheEntry = SceneCacheEntry & {
  scrollKey: string;
};

let SCENE_CACHE = new WeakMap<BattleBackgroundTilemap, SceneCacheEntry>();
let WRAPPED_SCENE_CACHE = new WeakMap<BattleBackgroundTilemap, WrappedSceneCacheEntry>();

const tileset_key = (tileset: Record<number, Record<number, Surface>>): string => {
  return `${Object.keys(tileset).length}`;
};

const size_key = (size: [number, number]): string => `${size[0]}x${size[1]}`;

export const invalidate_scene_cache = (): void => {
  SCENE_CACHE = new WeakMap();
  WRAPPED_SCENE_CACHE = new WeakMap();
};

export const render_battle_background = (
  surface: Surface,
  tilemap: BattleBackgroundTilemap,
  tileset: Record<number, Record<number, Surface>>,
  options?: { scx?: number; scy?: number; line_offsets_y?: number[] | null }
): void => {
  const scx = options?.scx ?? 0;
  const scy = options?.scy ?? 0;
  const lineOffsets = options?.line_offsets_y ?? null;
  const base = scene_surface(tilemap, tileset, [surface.width, surface.height]);
  const width = base.width;
  const height = base.height;
  const tilesetKey = tileset_key(tileset);
  const sizeKey = size_key([surface.width, surface.height]);
  const scrollKey = build_scroll_key(scx, scy, lineOffsets);
  const cached = WRAPPED_SCENE_CACHE.get(tilemap);
  if (
    cached &&
    cached.revision === tilemap.revision &&
    cached.tilesetKey === tilesetKey &&
    cached.sizeKey === sizeKey &&
    cached.scrollKey === scrollKey
  ) {
    surface.blit(cached.surface, [0, 0]);
    return;
  }

  const rendered = new Surface(surface.width, surface.height);
  render_wrapped_background(rendered, base, scx, scy, lineOffsets);
  WRAPPED_SCENE_CACHE.set(tilemap, {
    revision: tilemap.revision,
    tilesetKey,
    sizeKey,
    scrollKey,
    surface: rendered,
  });
  surface.blit(rendered, [0, 0]);
};

const render_wrapped_background = (
  surface: Surface,
  base: Surface,
  scx: number,
  scy: number,
  lineOffsets: number[] | null,
): void => {
  const width = base.width;
  const height = base.height;
  const offsetX = ((scx % width) + width) % width;
  if (!lineOffsets || lineOffsets.length === 0) {
    const offsetY = ((scy % height) + height) % height;
    for (const dx of [-offsetX, width - offsetX]) {
      for (const dy of [-offsetY, height - offsetY]) {
        surface.blit(base, [dx, dy]);
      }
    }
    return;
  }
  for (let y = 0; y < surface.height; y += 1) {
    const lineOffset = lineOffsets[y] ?? 0;
    const srcY = ((y + scy + lineOffset) % height + height) % height;
    const srcRect = new Rect(0, srcY, width, 1);
    for (const dx of [-offsetX, width - offsetX]) {
      surface.blit(base, [dx, y], srcRect);
    }
  }
};

const build_scroll_key = (scx: number, scy: number, lineOffsets: number[] | null): string => {
  if (!lineOffsets || lineOffsets.length === 0) {
    return `${scx & 0xffff}:${scy & 0xffff}:none`;
  }
  return `${scx & 0xffff}:${scy & 0xffff}:${lineOffsets.join(",")}`;
};

const scene_surface = (
  tilemap: BattleBackgroundTilemap,
  tileset: Record<number, Record<number, Surface>>,
  size: [number, number],
): Surface => {
  const cached = SCENE_CACHE.get(tilemap);
  const tilesetKey = tileset_key(tileset);
  const sizeKey = size_key(size);
  const revision = tilemap.revision;
  if (cached && cached.revision === revision && cached.tilesetKey === tilesetKey && cached.sizeKey === sizeKey) {
    return cached.surface;
  }
  const surface = new Surface(size[0], size[1]);
  tilemap.blit(surface, tileset);
  SCENE_CACHE.set(tilemap, { revision, tilesetKey, sizeKey, surface });
  return surface;
};

const convert_bitmap_font_tiles = (
  tiles: Record<number, InstanceType<typeof gameEngine.Surface>>
): Record<number, Surface> => {
  const converted: Record<number, Surface> = {};
  for (const [tileIdRaw, tileSurface] of Object.entries(tiles)) {
    if (!tileSurface) {
      continue;
    }
    converted[Number(tileIdRaw)] = Surface.fromImageData(tileSurface.getImageData());
  }
  return converted;
};

export const render_default_scene = (surface: Surface): void => {
  const font = new BitmapFont();
  // BitmapFont uses gameEngine surfaces; battle tileset expects the UI surface shape.
  const [tileset] = build_battle_tileset(convert_bitmap_font_tiles(font.font_tiles));
  const layout = BattleUILayoutFactory.fromAsmDefaults();
  const tilemap = build_battle_tilemap(layout);
  tilemap.draw_window(
    layout.text_box.tile_x,
    layout.text_box.tile_y,
    layout.text_box.width_tiles,
    layout.text_box.height_tiles,
    { attr: PAL_TEXT_WINDOW },
  );
  if (layout.menu_window) {
    tilemap.draw_window(
      layout.menu_window.tile_x,
      layout.menu_window.tile_y,
      layout.menu_window.width_tiles,
      layout.menu_window.height_tiles,
      { attr: PAL_MENU },
    );
  }
  render_battle_background(surface, tilemap, tileset);
};
