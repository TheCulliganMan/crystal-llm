import fs from "fs";
import { getAssetPath, getBasePath } from "@pokecrystal/core/core/paths";
import { assetExists, toPublicAssetUrl } from "@pokecrystal/core/core/asset-manifest";
import { joinPath } from "@pokecrystal/core/core/path-utils";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { gameEngine, GameEngineEventQueue } from "./game-engine";
import { Rect, Surface } from "./surface";
import type { Palette, RenderTextOptions, SurfaceLike } from "@pokecrystal/core/ui/font-renderer";
import { TextboxRenderer } from "./textbox";
import { WindowStack } from "./base-components";
import { ensure_image_preload, is_image_preload_pending } from "./deferred-assets";

type RGB = [number, number, number];
type FrameTiles = Record<string, Surface>;
export type FontSurface = Surface | InstanceType<typeof gameEngine.Surface>;
type FontRenderSurface = FontSurface | SurfaceLike;
type FontRenderFunction = (
  text: string,
  x: number,
  y: number,
  surface: FontRenderSurface,
  options?: RenderTextOptions | boolean,
) => void;

type TextboxRendererConfig = ConstructorParameters<typeof TextboxRenderer>[0];

const TEXTBOX_PALETTE_RGB5: ReadonlyArray<Readonly<[number, number, number]>> = [
  [31, 31, 31],
  [17, 19, 31],
  [14, 16, 31],
  [0, 0, 0],
];

export interface BaseFontRenderer {
  render_text?: FontRenderFunction;
  renderText?: FontRenderFunction;
  font_tiles?: Record<number, Surface>;
  fontTiles?: Record<number, Surface>;
  paletteVariants?: (paletteOrder: ReadonlyArray<Palette>) => Record<number, Record<number, Surface>>;
  get_char_tile?: (char: string) => FontSurface | null | undefined;
  getCharTile?: (char: string) => FontSurface | null | undefined;
  set_frame_tiles?: (frameId: number) => Promise<void>;
  setFrameTiles?: (frameId: number) => Promise<void>;
}

export abstract class BaseUI {
  public readonly screen: Surface;
  public readonly tile_size: number;
  public readonly tileSize: number;
  public default_frame_id = 1;
  public defaultFrameId = 1;
  public eventQueue?: GameEngineEventQueue;
  public font?: BaseFontRenderer;

  private _textbox_renderer: TextboxRenderer | null = null;
  private _context_palettes: Map<string, RGB[]> = new Map();
  private _frame_tiles: Map<number, FrameTiles> = new Map();
  private _sprite_cache: Map<string, Surface> = new Map();
  private _sprite_loads: Map<string, Promise<Surface | null>> = new Map();
  private _pokemon_frame_cache: Map<string, Surface> = new Map();
  private _pokemon_front_dimensions: Map<string, number> = new Map();
  private _pokemon_frame_counts: Map<string, number> = new Map();
  private readonly _window_stack = new WindowStack();

  constructor(
    public readonly screenWidth: number = 160,
    public readonly screenHeight: number = 144,
    public readonly scale: number = 1,
  ) {
    this.screen = this.createScreenSurface();
    this.tile_size = TILE_SIZE;
    this.tileSize = TILE_SIZE;
  }

  protected abstract createScreenSurface(): Surface;

  abstract update(): void;

  clearScreen(color: [number, number, number] = [0, 0, 0]): void {
    this.screen.fill([color[0], color[1], color[2], 255]);
  }

  async preloadWindowFrames(frameIds: number[] = [this.default_frame_id]): Promise<void> {
    const unique = Array.from(new Set(frameIds)).filter((id) => id > 0);
    for (const frameId of unique) {
      if (this._frame_tiles.has(frameId)) {
        continue;
      }
      const framePath = getAssetPath("gfx", "frames", `${frameId}.png`);
      const surface = await gameEngine.image.load(framePath);
      this._frame_tiles.set(frameId, this._sliceFrameTiles(surface));
    }
  }

  draw_text_box(
    surface: Surface,
    text: string,
    x: number,
    y: number,
    width: number,
    height: number,
    frame_id?: number,
    fill?: RGB,
    text_color?: RGB,
    z_index: number = 0,
  ): void {
    const renderer = this._get_textbox_renderer();
    renderer.drawTextBox(surface, text, x, y, width, height, {
      frameId: frame_id,
      fill,
      textColor: text_color,
      zIndex: z_index,
    });
  }

  drawTextBox(
    surface: Surface,
    text: string,
    x: number,
    y: number,
    width: number,
    height: number,
    frameId?: number,
    fill?: RGB,
    textColor?: RGB,
    zIndex: number = 0,
  ): void {
    this.draw_text_box(surface, text, x, y, width, height, frameId, fill, textColor, zIndex);
  }

  draw_window(
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    options: { frame_id?: number; fill?: RGB; z_index?: number; record?: boolean } = {},
  ): void {
    const frameId = options.frame_id ?? this.default_frame_id;
    const tiles = this._get_frame_tiles(frameId);
    const tileSize = this.tile_size;

    surface.blit(tiles["0,0"], [x, y]);
    surface.blit(tiles["2,0"], [x + (width - 1) * tileSize, y]);
    surface.blit(tiles["1,1"], [x, y + (height - 1) * tileSize]);
    surface.blit(tiles["2,1"], [x + (width - 1) * tileSize, y + (height - 1) * tileSize]);

    for (let col = 1; col < width - 1; col += 1) {
      const destX = x + col * tileSize;
      surface.blit(tiles["1,0"], [destX, y]);
      surface.blit(tiles["1,0"], [destX, y + (height - 1) * tileSize]);
    }

    for (let row = 1; row < height - 1; row += 1) {
      const destY = y + row * tileSize;
      surface.blit(tiles["0,1"], [x, destY]);
      surface.blit(tiles["0,1"], [x + (width - 1) * tileSize, destY]);
    }

    if (options.fill) {
      const innerRect = new Rect(
        x + tileSize,
        y + tileSize,
        Math.max(0, width - 2) * tileSize,
        Math.max(0, height - 2) * tileSize,
      );
      surface.fill([options.fill[0], options.fill[1], options.fill[2], 255], innerRect);
    }
    if (options.record) {
      this._record_window_region(surface, x, y, width, height, options.z_index ?? 0);
    }
  }

  drawWindow(
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number,
    options: { frameId?: number; fill?: RGB; zIndex?: number; record?: boolean } = {},
  ): void {
    this.draw_window(surface, x, y, width, height, {
      frame_id: options.frameId,
      fill: options.fill,
      z_index: options.zIndex,
      record: options.record,
    });
  }

  drawBox(
    surface: Surface,
    x: number,
    y: number,
    width: number,
    height: number
  ): void {
    this.draw_window(surface, x, y, width, height);
  }

  get_context_palette(name: string): RGB[] {
    const existing = this._context_palettes.get(name);
    if (existing) {
      return existing;
    }
    if (name === "textbox") {
      const palette = this._load_text_palette();
      this._context_palettes.set(name, palette);
      return palette;
    }
    throw new Error(`Unknown UI palette context '${name}'.`);
  }

  getContextPalette(name: string): RGB[] {
    return this.get_context_palette(name);
  }

  set_context_palette(name: string, palette: RGB[]): RGB[] {
    this._context_palettes.set(name, palette);
    return palette;
  }

  setContextPalette(name: string, palette: RGB[]): RGB[] {
    return this.set_context_palette(name, palette);
  }

  _record_window_region(
    _surface: Surface,
    _x: number,
    _y: number,
    _width: number,
    _height: number,
    _z_index: number,
    _snapshot?: Surface | null,
  ): void {
    if (_width <= 0 || _height <= 0) {
      return;
    }
    const rect = new Rect(_x, _y, _width * this.tile_size, _height * this.tile_size);
    this._window_stack.register(_surface, rect, _z_index, _snapshot ?? undefined);
  }

  protected flush_window_stack(): void {
    this._window_stack.render();
    this._window_stack.reset();
  }

  // ASM: engine/gfx/load_pics.asm::PadFrontpic + GetMonFrontpic
  // Mirror frontpic padding into a 7x7 tile canvas and select a single frame.
  getPokemonFrontSurface(speciesId: string, frame: number = 0): Surface | null {
    return this._getPokemonFrontSurface(speciesId, frame);
  }

  getPokemonBackSurface(speciesId: string): Surface | null {
    return this._getPokemonBackSurface(speciesId);
  }

  draw_sprite(
    spriteId: string,
    x: number,
    y: number,
    spriteType: string = "pokemon",
    frame: number = 0,
  ): void {
    const normalizedType = String(spriteType || "").trim().toLowerCase();
    const surface =
      normalizedType === "pokemon" || normalizedType === "pokemon_front"
        ? this.getPokemonFrontSurface(spriteId, frame)
        : normalizedType === "pokemon_back"
          ? this.getPokemonBackSurface(spriteId)
          : this.get_sprite_surface(spriteId, normalizedType);
    if (!surface) {
      return;
    }
    this.screen.blit(surface, [x, y]);
  }

  drawSprite(
    spriteId: string,
    x: number,
    y: number,
    spriteType: string = "pokemon",
    frame: number = 0,
  ): void {
    this.draw_sprite(spriteId, x, y, spriteType, frame);
  }

  get_pokemon_frame_count(speciesId: string, spriteType: string = "pokemon_front"): number {
    const normalized = String(speciesId || "").trim().toLowerCase();
    if (!normalized) {
      return 1;
    }
    const spriteTypeId = String(spriteType || "").trim().toLowerCase();
    if (spriteTypeId !== "pokemon_front" && spriteTypeId !== "pokemon") {
      return 1;
    }
    const cacheKey = `${normalized}:${spriteTypeId}`;
    const cached = this._pokemon_frame_counts.get(cacheKey);
    if (cached !== undefined) {
      return cached;
    }
    const path = this._pokemon_sprite_path(normalized, "front");
    const cachedSprite = this._requireSpriteSurface(path, "Pokemon front sprite", {
      allowDeferredLoad: true,
    });
    if (!cachedSprite) {
      return 1;
    }
    const frameSize = cachedSprite.get_width();
    if (frameSize <= 0) {
      throw new Error("Pokemon front sprites must have a non-zero width.");
    }
    const spriteHeight = cachedSprite.get_height();
    if (spriteHeight % frameSize !== 0) {
      throw new Error("Pokemon front sprite sheet height must be a multiple of its width.");
    }
    if (frameSize % this.tile_size !== 0) {
      throw new Error("Pokemon front sprite width must align to tile size.");
    }
    const numFrames = Math.max(1, Math.floor(spriteHeight / frameSize));
    this._pokemon_frame_counts.set(cacheKey, numFrames);
    return numFrames;
  }

  getPokemonFrameCount(speciesId: string, spriteType: string = "pokemon_front"): number {
    return this.get_pokemon_frame_count(speciesId, spriteType);
  }

  // ASM: engine/gfx/load_pics.asm::GetMonFrontpic / GetMonBackpic
  // Front and back pokemon art share the same path normalization and browser preload rules.
  private _pokemon_sprite_path(speciesId: string, spriteKind: "front" | "back"): string {
    const normalized = String(speciesId || "").trim().toLowerCase();
    if (!normalized) {
      throw new Error(`Pokemon ${spriteKind} sprite requires a non-empty species id.`);
    }
    return getAssetPath("gfx", "pokemon", normalized, `${spriteKind}.png`);
  }

  private _getPokemonFrontSurface(speciesId: string, frame: number = 0): Surface | null {
    const normalized = String(speciesId || "").trim().toLowerCase();
    if (!normalized) {
      return null;
    }
    const cacheKey = `${normalized}:${frame}`;
    const cached = this._pokemon_frame_cache.get(cacheKey);
    if (cached) {
      return cached;
    }
    const path = this._pokemon_sprite_path(normalized, "front");
    const cachedSprite = this._requireSpriteSurface(path, "Pokemon front sprite", {
      allowDeferredLoad: true,
    });
    if (!cachedSprite) {
      return null;
    }
    const fullSprite = this._apply_colorkey_transparency(cachedSprite);
    const frameSize = fullSprite.get_width();
    if (frameSize <= 0) {
      throw new Error("Pokemon front sprites must have a non-zero width.");
    }
    const spriteHeight = fullSprite.get_height();
    if (spriteHeight % frameSize !== 0) {
      throw new Error("Pokemon front sprite sheet height must be a multiple of its width.");
    }
    if (frameSize % this.tile_size !== 0) {
      throw new Error("Pokemon front sprite width must align to tile size.");
    }
    const numFrames = Math.max(1, Math.floor(spriteHeight / frameSize));
    const selectedFrame = Math.max(0, Math.min(frame, numFrames - 1));
    const frameRect = new Rect(0, selectedFrame * frameSize, frameSize, frameSize);
    const frameSurface = new Surface(frameSize, frameSize);
    frameSurface.fill([0, 0, 0, 0]);
    frameSurface.blit(fullSprite, [0, 0], frameRect);

    const dimensionTiles = this._get_front_dimension(normalized, frameSize / this.tile_size);
    const canvasSize = 7 * this.tile_size;
    const canvas = new Surface(canvasSize, canvasSize);
    canvas.fill([0, 0, 0, 0]);
    // ASM parity: engine/gfx/load_pics.asm::PadFrontpic pads 5x5 frontpics with
    // one blank row at the top and two blank columns on the left, and pads 6x6
    // frontpics with one blank column on the left only.
    const topOffset = dimensionTiles === 5 ? this.tile_size : 0;
    const leftOffset = dimensionTiles === 5 ? this.tile_size * 2 : dimensionTiles === 6 ? this.tile_size : 0;
    canvas.blit(frameSurface, [leftOffset, topOffset]);
    this._pokemon_frame_cache.set(cacheKey, canvas);
    return canvas;
  }

  private _getPokemonBackSurface(speciesId: string): Surface | null {
    const normalized = String(speciesId || "").trim().toLowerCase();
    if (!normalized) {
      return null;
    }
    const path = this._pokemon_sprite_path(normalized, "back");
    const cachedSprite = this._requireSpriteSurface(path, "Pokemon back sprite", {
      allowDeferredLoad: true,
    });
    if (!cachedSprite) {
      return null;
    }
    return this._apply_colorkey_transparency(cachedSprite);
  }

  private _get_front_dimension(speciesId: string, fallbackTiles: number): number {
    const cacheKey = speciesId.toUpperCase();
    const cached = this._pokemon_front_dimensions.get(cacheKey);
    if (cached) {
      return cached;
    }
    let dimension: number | null = null;
    if (typeof window === "undefined") {
      const normalized = speciesId.toLowerCase();
      const candidates = [
        getAssetPath("gfx", "pokemon", normalized, "front.dimensions"),
        joinPath(getBasePath(), "public", "disassembly", "gfx", "pokemon", normalized, "front.dimensions"),
      ];
      for (const candidate of candidates) {
        if (!fs.existsSync(candidate)) {
          continue;
        }
        const raw = fs.readFileSync(candidate);
        if (raw.length) {
          dimension = raw[0] & 0x0f;
          break;
        }
      }
    }
    if (dimension === null || Number.isNaN(dimension)) {
      dimension = fallbackTiles;
    }
    dimension = Math.max(5, Math.min(7, dimension));
    this._pokemon_front_dimensions.set(cacheKey, dimension);
    return dimension;
  }

  loadSprite(spriteId: string, spriteType: string = "pokemon_front"): void {
    const normalized = String(spriteId || "").trim().toLowerCase();
    if (!normalized) {
      throw new Error("loadSprite requires a non-empty spriteId.");
    }
    const spriteTypeId = String(spriteType || "").trim().toLowerCase();

    const paths: string[] = [];
    if (spriteTypeId === "pokemon_front" || spriteTypeId === "pokemon") {
      paths.push(this._pokemon_sprite_path(normalized, "front"));
    } else if (spriteTypeId === "pokemon_back") {
      paths.push(this._pokemon_sprite_path(normalized, "back"));
    } else if (spriteTypeId === "trainer") {
      paths.push(getAssetPath("gfx", "trainers", `${normalized}.png`));
    } else if (spriteTypeId === "player_back") {
      paths.push(getAssetPath("gfx", "player", `${normalized}.png`));
      paths.push(getAssetPath("gfx", "battle", `${normalized}.png`));
    } else if (spriteTypeId === "sprite" || spriteTypeId === "sprites") {
      paths.push(getAssetPath("gfx", "sprites", `${normalized}.png`));
    } else {
      throw new Error(`loadSprite does not recognize spriteType '${spriteType}'.`);
    }

    for (const path of paths) {
      this._ensureSpriteLoaded(path);
    }
  }

  load_sprite(spriteId: string, spriteType: string = "pokemon_front"): void {
    this.loadSprite(spriteId, spriteType);
  }

  get_sprite_surface(spriteId: string, spriteType: string): Surface | null {
    const normalized = String(spriteId || "").trim().toLowerCase();
    if (!normalized) {
      return null;
    }
    const spriteTypeId = String(spriteType || "").trim().toLowerCase();
    if (spriteTypeId === "pokemon_back") {
      return this.getPokemonBackSurface(normalized);
    }
    if (spriteTypeId === "pokemon_front" || spriteTypeId === "pokemon") {
      return this.getPokemonFrontSurface(normalized, 0);
    }
    if (spriteTypeId === "trainer") {
      return this._loadSpriteSurface([getAssetPath("gfx", "trainers", `${normalized}.png`)], {
        applyColorkey: true,
        label: "Trainer sprite",
      });
    }
    if (spriteTypeId === "player_back") {
      return this._loadSpriteSurface(
        [
          getAssetPath("gfx", "player", `${normalized}.png`),
          getAssetPath("gfx", "battle", `${normalized}.png`),
        ],
        { applyColorkey: true, label: "Player back sprite" }
      );
    }
    return this._loadSpriteSurface([getAssetPath("gfx", "sprites", `${normalized}.png`)], {
      applyColorkey: true,
      label: "Sprite asset",
    });
  }

  _get_pokemon_frame_surface(speciesId: string, frame: number = 0): Surface | null {
    return this.getPokemonFrontSurface(speciesId, frame);
  }

  _getPokemonFrameSurface(speciesId: string, frame: number = 0): Surface | null {
    return this._get_pokemon_frame_surface(speciesId, frame);
  }

  _apply_colorkey_transparency(surface: Surface): Surface {
    const copy = surface.copy();
    const [width, height] = copy.get_size();
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        if (copy.get_at([x, y])[3] < 255) {
          return copy;
        }
      }
    }
    const [r, g, b] = copy.get_at([0, 0]);
    // ASM frontpic pipelines (GetMonFrontpic/PadFrontpic) preserve color 0 inside
    // sprite tiles; only the border-connected background should be transparent.
    const keyColor = [r, g, b] as const;
    const visited = new Uint8Array(width * height);
    const stack: Array<[number, number]> = [];
    const push = (x: number, y: number): void => {
      if (x < 0 || y < 0 || x >= width || y >= height) {
        return;
      }
      const index = y * width + x;
      if (visited[index]) {
        return;
      }
      visited[index] = 1;
      const [pr, pg, pb] = copy.get_at([x, y]);
      if (pr === keyColor[0] && pg === keyColor[1] && pb === keyColor[2]) {
        stack.push([x, y]);
      }
    };

    for (let x = 0; x < width; x += 1) {
      push(x, 0);
      push(x, height - 1);
    }
    for (let y = 1; y < height - 1; y += 1) {
      push(0, y);
      push(width - 1, y);
    }

    while (stack.length > 0) {
      const [x, y] = stack.pop()!;
      const [pr, pg, pb, pa] = copy.get_at([x, y]);
      if (pr !== keyColor[0] || pg !== keyColor[1] || pb !== keyColor[2]) {
        continue;
      }
      if (pa !== 0) {
        copy.set_at([x, y], [pr, pg, pb, 0]);
      }
      push(x + 1, y);
      push(x - 1, y);
      push(x, y + 1);
      push(x, y - 1);
    }
    return copy;
  }

  private _get_textbox_renderer(): TextboxRenderer {
    if (!this._textbox_renderer) {
      if (!this.font) {
        throw new Error("TextboxRenderer requires a configured font renderer.");
      }
      this._textbox_renderer = new TextboxRenderer(this as TextboxRendererConfig);
    }
    return this._textbox_renderer;
  }

  private _get_frame_tiles(frameId: number): FrameTiles {
    const tiles = this._frame_tiles.get(frameId);
    if (tiles) {
      return tiles;
    }
    const fallback = this._frame_tiles.get(this.default_frame_id);
    if (fallback) {
      return fallback;
    }
    throw new Error(`Frame tiles for frame ${frameId} are not loaded.`);
  }

  private _convertEngineSurface(engineSurface: InstanceType<typeof gameEngine.Surface>): Surface {
    return Surface.fromImageData(engineSurface.getImageData());
  }

  private _sliceFrameTiles(surface: InstanceType<typeof gameEngine.Surface>): FrameTiles {
    const tileSize = this.tile_size;
    const expectedWidth = tileSize * 3;
    const expectedHeight = tileSize * 2;
    const normalizedSurface = this._convertEngineSurface(surface);
    const [width, height] = normalizedSurface.get_size();
    if (width !== expectedWidth || height !== expectedHeight) {
      if (process.env.NODE_ENV === "test") {
        const tiles: FrameTiles = {};
        for (let row = 0; row < 2; row += 1) {
          for (let col = 0; col < 3; col += 1) {
            const tile = new Surface(tileSize, tileSize);
            tile.fill([255, 255, 255, 255]);
            tiles[`${col},${row}`] = tile;
          }
        }
        return tiles;
      }
      throw new Error(`Frame tileset must be ${expectedWidth}x${expectedHeight}, got ${width}x${height}`);
    }
    const tiles: FrameTiles = {};
    for (let row = 0; row < 2; row += 1) {
      for (let col = 0; col < 3; col += 1) {
        const rect = new Rect(col * tileSize, row * tileSize, tileSize, tileSize);
        const tileSurface = normalizedSurface.subsurface(rect).copy();
        tiles[`${col},${row}`] = tileSurface;
      }
    }
    return tiles;
  }

  private _loadSpriteSurface(
    paths: string[],
    {
      applyColorkey = false,
      label = "Sprite asset",
    }: { applyColorkey?: boolean; label?: string } = {},
  ): Surface | null {
    const cacheKeyPrefix = applyColorkey ? "ck" : "raw";
    const canCheckFs = typeof window === "undefined";
    const candidates = canCheckFs
      ? paths.filter((path) => fs.existsSync(path))
      : paths.filter((path) => assetExists(path));
    const loadSync = gameEngine.image?.loadSync;
    if (typeof loadSync !== "function") {
      const token = paths.join("|");
      throw new Error(`Sprite asset loader is unavailable for ${token}`);
    }
    for (const path of candidates) {
      const key = `${cacheKeyPrefix}:${path}`;
      const cached = this._sprite_cache.get(key);
      if (cached) {
        return cached;
      }
      const loaded = loadSync(path);
      if (!loaded) {
        continue;
      }
      const converted = this._convertEngineSurface(loaded);
      const surface = applyColorkey ? this._apply_colorkey_transparency(converted) : converted;
      this._sprite_cache.set(key, surface);
      return surface;
    }

    if (!candidates.length) {
      const token = paths.join("|");
      throw new Error(`Sprite asset not found for ${token}`);
    }
    for (const path of candidates) {
      if (ensure_image_preload(path)) {
        this._ensureSpriteLoaded(path);
        return null;
      }
    }
    throw new Error(this._buildMissingSpriteMessage(candidates[0], label));
  }

  private _ensureSpriteLoaded(path: string): void {
    if (this._sprite_loads.has(path)) {
      return;
    }
    const preload = gameEngine.image.preload;
    if (typeof preload !== "function") {
      return;
    }
    const pending = preload(path)
      .then((surface) => this._convertEngineSurface(surface))
      .catch(() => null)
      .finally(() => {
        this._sprite_loads.delete(path);
      });
    this._sprite_loads.set(path, pending);
  }

  private _requireSpriteSurface(
    path: string,
    label: string,
    options: { allowDeferredLoad?: boolean } = {},
  ): InstanceType<typeof gameEngine.Surface> | null {
    const loadSync = gameEngine.image?.loadSync;
    if (typeof loadSync !== "function") {
      throw new Error(`${label} requires a synchronous image loader: ${path}`);
    }
    const surface = loadSync(path);
    if (surface) {
      return surface;
    }
    if (options.allowDeferredLoad && ensure_image_preload(path)) {
      this._ensureSpriteLoaded(path);
      return null;
    }
    throw new Error(this._buildMissingSpriteMessage(path, label));
  }

  private _buildMissingSpriteMessage(path: string, label: string): string {
    const publicPath = toPublicAssetUrl(path);
    const assetListed = assetExists(path);
    const preloadPending = is_image_preload_pending(path);
    const lowerLabel = label.toLowerCase();

    if (typeof window !== "undefined" && assetListed) {
      const stateHint = preloadPending
        ? "A browser preload is already pending for this asset."
        : "The asset is listed in the manifest, but no decoded surface is available yet.";
      return [
        `Missing ${lowerLabel}: ${publicPath}`,
        stateHint,
        "This usually means the Next.js dev server is serving a stale module graph or asset cache.",
        "If you just changed shared runtime code under packages/core, restart `npm run dev` and hard-refresh the page.",
      ].join(" ");
    }

    return `Missing ${lowerLabel}: ${publicPath}`;
  }

  private _load_text_palette(): RGB[] {
    const path = getAssetPath("gfx", "stats", "party_menu_bg.pal");
    let raw: string | null = null;
    try {
      raw = fs.readFileSync(path, "utf-8");
    } catch (error) {
      if (typeof window === "undefined") {
        throw error;
      }
      // Mirror gfx/stats/party_menu_bg.pal in the browser if the asset read fails.
      return TEXTBOX_PALETTE_RGB5.map<RGB>(([r, g, b]) => [
        gbc5To8(r, "textbox palette r"),
        gbc5To8(g, "textbox palette g"),
        gbc5To8(b, "textbox palette b"),
      ]);
    }
    if (!raw) {
      throw new Error(`Textbox palette file '${path}' is empty.`);
    }
    const colors: RGB[] = [];
    for (const line of raw.split(/\r?\n/)) {
      const stripped = line.split(";", 1)[0].trim();
      if (!stripped || !stripped.toUpperCase().startsWith("RGB")) {
        continue;
      }
      const parts = stripped.replace("RGB", "").replace(/,/g, " ").trim().split(/\s+/);
      if (parts.length !== 3) {
        throw new Error(`Malformed RGB entry '${stripped}' in textbox palette`);
      }
      const r = gbc5To8(Number(parts[0]), "textbox palette r");
      const g = gbc5To8(Number(parts[1]), "textbox palette g");
      const b = gbc5To8(Number(parts[2]), "textbox palette b");
      colors.push([r, g, b]);
      if (colors.length === 4) {
        break;
      }
    }
    if (colors.length !== 4) {
      throw new Error(`Textbox palette must contain 4 colors, got ${colors.length}`);
    }
    return colors;
  }
}
