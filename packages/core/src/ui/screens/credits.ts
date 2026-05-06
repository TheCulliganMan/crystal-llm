import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { buttonKeys, isKeyDownEvent, KeyEvent } from "@pokecrystal/core/input/controls";
import { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";
import {
  CreditsOp,
  PaletteSet,
  loadCreditConstantIndices,
  loadCreditsPalettes,
  loadCreditsScript,
  loadCreditsStringTiles,
} from "@pokecrystal/core/ui/screens/credits-data";

// ASM reference: engine/movie/credits.asm.
// Key entrypoints mirrored:
// - ConstructCreditsTilemap
// - Credits_Jumptable and Credits_LoadBorderGFX
// - ParseCredits

type CreditsAudioEngine = AudioEngine & {
  skipMusicFrames?: (frames: number) => void;
  fadeOut?: (name: string, frames: number) => void;
};

const paletteIndexFromGray = (value: number): number => {
  if (value < 64) return 0;
  if (value < 128) return 1;
  if (value < 192) return 2;
  return 3;
};

const createPlaceholderSurface = (
  width: number,
  height: number,
  color: [number, number, number] = [0, 0, 0]
): gameEngine.Surface => {
  const surface = new gameEngine.Surface(width, height);
  surface.fill([color[0], color[1], color[2], 255]);
  return surface;
};

export class CreditsGraphics {
  static readonly TILE_SIZE = 8;
  static readonly MON_FRAME_SIZE = 32;
  static readonly BORDER_TILES = 9;
  static readonly FRAMES_PER_SCENE = 4;

  static readonly BG_PALETTE_INDEX = 0;
  static readonly BORDER_PALETTE_INDEX = 1;
  static readonly TEXT_PALETTE_INDEX = 2;

  static readonly COPYRIGHT_TILE_BASE = 0x60;
  static readonly COPYRIGHT_TILE_COUNT = 29;
  static readonly THE_END_WIDTH = 64;
  static readonly THE_END_HEIGHT = 16;

  readonly paletteSets: PaletteSet[];
  private readonly borderTilesGray: gameEngine.Surface[];
  private readonly theEndGray: gameEngine.Surface;
  private readonly copyrightTilesGray: gameEngine.Surface[];
  private readonly monFramesGray: Record<string, gameEngine.Surface[]>;
  private readonly borderCache = new Map<string, gameEngine.Surface>();
  private readonly frameCache = new Map<string, gameEngine.Surface>();
  private readonly theEndCache = new Map<number, gameEngine.Surface>();
  private readonly blankCache = new Map<number, gameEngine.Surface>();
  private readonly copyrightCache = new Map<number, gameEngine.Surface[]>();

  private constructor(
    paletteSets: PaletteSet[],
    borderTilesGray: gameEngine.Surface[],
    theEndGray: gameEngine.Surface,
    copyrightTilesGray: gameEngine.Surface[],
    monFramesGray: Record<string, gameEngine.Surface[]>
  ) {
    this.paletteSets = paletteSets;
    this.validatePaletteSets();
    this.borderTilesGray = borderTilesGray;
    this.theEndGray = theEndGray;
    this.copyrightTilesGray = copyrightTilesGray;
    this.monFramesGray = monFramesGray;
    if (this.copyrightTilesGray.length !== CreditsGraphics.COPYRIGHT_TILE_COUNT) {
      throw new Error(
        `Credits copyright tiles expected ${CreditsGraphics.COPYRIGHT_TILE_COUNT} tiles, got ${this.copyrightTilesGray.length}.`
      );
    }
  }

  static async create(): Promise<CreditsGraphics> {
    const paletteSets = loadCreditsPalettes();
    const borderTilesGray = await CreditsGraphics.loadTiles("border.png", 3, 3);
    const theEndGray = await CreditsGraphics.loadSurface(
      "theend.png",
      "credits",
      CreditsGraphics.THE_END_WIDTH,
      CreditsGraphics.THE_END_HEIGHT
    );
    const copyrightTilesGray = await CreditsGraphics.loadTiles(
      "copyright.png",
      CreditsGraphics.COPYRIGHT_TILE_COUNT,
      1,
      "splash"
    );
    const monFramesGray: Record<string, gameEngine.Surface[]> = {
      pichu: await CreditsGraphics.loadFrames("pichu.png"),
      smoochum: await CreditsGraphics.loadFrames("smoochum.png"),
      ditto: await CreditsGraphics.loadFrames("ditto.png"),
      igglybuff: await CreditsGraphics.loadFrames("igglybuff.png"),
    };
    return new CreditsGraphics(
      paletteSets,
      borderTilesGray,
      theEndGray,
      copyrightTilesGray,
      monFramesGray
    );
  }

  private validatePaletteSets(): void {
    if (!this.paletteSets.length) {
      throw new Error("Credits palettes were not loaded.");
    }
    if (this.paletteSets.length !== 4) {
      throw new Error(
        `Credits palettes should contain exactly 4 scene sets, got ${this.paletteSets.length}.`
      );
    }
    this.paletteSets.forEach((paletteSet, idx) => {
      if (paletteSet.length !== 3) {
        throw new Error(`Credits palette set ${idx} must contain 3 palettes.`);
      }
      paletteSet.forEach((palette, paletteIdx) => {
        if (palette.length !== 4) {
          throw new Error(
            `Credits palette ${idx}:${paletteIdx} must contain 4 colours.`
          );
        }
      });
    });
  }

  getPaletteSet(sceneIndex: number): PaletteSet {
    const normalized = sceneIndex & 0x03;
    return this.paletteSets[normalized];
  }

  getBorderTiles(sceneIndex: number, paletteIndex = 1): gameEngine.Surface[] {
    const palette = this.getPaletteSet(sceneIndex)[paletteIndex];
    const tiles: gameEngine.Surface[] = [];
    this.borderTilesGray.forEach((tile, idx) => {
      const cacheKey = `${sceneIndex}:${paletteIndex}:${idx}`;
      const cached = this.borderCache.get(cacheKey);
      if (cached) {
        tiles.push(cached);
        return;
      }
      const tinted = this.tintSurface(tile, palette);
      this.borderCache.set(cacheKey, tinted);
      tiles.push(tinted);
    });
    return tiles;
  }

  getMonFrame(
    sceneIndex: number,
    frameIndex: number,
    monIndex?: number
  ): gameEngine.Surface | null {
    const monKey = CreditsGraphics.sceneToMon(
      monIndex === undefined ? sceneIndex : monIndex
    );
    const frames = this.monFramesGray[monKey];
    if (!frames || !frames.length) {
      return null;
    }
    const palette = this.getPaletteSet(sceneIndex)[CreditsGraphics.BG_PALETTE_INDEX];
    const normalized = frameIndex % CreditsGraphics.FRAMES_PER_SCENE;
    const cacheKey = `${sceneIndex}:${monKey}:${normalized}`;
    const cached = this.frameCache.get(cacheKey);
    if (cached) {
      return cached;
    }
    const tinted = this.tintSurface(frames[normalized], palette);
    this.frameCache.set(cacheKey, tinted);
    return tinted;
  }

  getBlankFrame(sceneIndex: number): gameEngine.Surface {
    const cached = this.blankCache.get(sceneIndex);
    if (cached) {
      return cached;
    }
    const palette = this.getPaletteSet(sceneIndex)[CreditsGraphics.BG_PALETTE_INDEX];
    const surface = createPlaceholderSurface(
      CreditsGraphics.MON_FRAME_SIZE,
      CreditsGraphics.MON_FRAME_SIZE,
      palette[2]
    );
    this.blankCache.set(sceneIndex, surface);
    return surface;
  }

  getTheEnd(sceneIndex: number): gameEngine.Surface {
    const cached = this.theEndCache.get(sceneIndex);
    if (cached) {
      return cached;
    }
    const palette = this.getPaletteSet(sceneIndex)[CreditsGraphics.TEXT_PALETTE_INDEX];
    const tinted = this.tintSurface(this.theEndGray, palette, true);
    this.theEndCache.set(sceneIndex, tinted);
    return tinted;
  }

  getCopyrightTiles(sceneIndex: number): gameEngine.Surface[] {
    const cached = this.copyrightCache.get(sceneIndex);
    if (cached) {
      return cached;
    }
    const palette = this.getPaletteSet(sceneIndex)[CreditsGraphics.TEXT_PALETTE_INDEX];
    const tinted = this.copyrightTilesGray.map((tile) =>
      this.tintSurface(tile, palette)
    );
    this.copyrightCache.set(sceneIndex, tinted);
    return tinted;
  }

  private static async loadSurface(
    filename: string,
    subdir = "credits",
    expectedWidth?: number,
    expectedHeight?: number
  ): Promise<gameEngine.Surface> {
    const { getAssetPath } = await import("@pokecrystal/core/core/paths");
    const assetPath = getAssetPath("gfx", subdir, filename);
    const surface = await gameEngine.image.load(assetPath);
    if (expectedWidth !== undefined || expectedHeight !== undefined) {
      if (expectedWidth === undefined || expectedHeight === undefined) {
        throw new Error(
          `Partial size assertion for credits asset ${filename}: both width and height must be provided.`
        );
      }
      const width = surface.get_width();
      const height = surface.get_height();
      if (width !== expectedWidth || height !== expectedHeight) {
        throw new Error(
          `Credits asset ${filename} must be ${expectedWidth}x${expectedHeight}, got ${width}x${height}.`
        );
      }
    }
    return surface;
  }

  private static async loadTiles(
    filename: string,
    tilesWide: number,
    tilesHigh: number,
    subdir = "credits"
  ): Promise<gameEngine.Surface[]> {
    const surface = await CreditsGraphics.loadSurface(filename, subdir);
    const expectedWidth = tilesWide * CreditsGraphics.TILE_SIZE;
    const expectedHeight = tilesHigh * CreditsGraphics.TILE_SIZE;
    const width = surface.get_width();
    const height = surface.get_height();
    if (width !== expectedWidth || height !== expectedHeight) {
      throw new Error(
        `Credits tile asset ${filename} must be ${expectedWidth}x${expectedHeight}, got ${width}x${height}.`
      );
    }
    const tiles: gameEngine.Surface[] = [];
    for (let tileY = 0; tileY < tilesHigh; tileY += 1) {
      for (let tileX = 0; tileX < tilesWide; tileX += 1) {
        const rect = new gameEngine.Rect(
          tileX * CreditsGraphics.TILE_SIZE,
          tileY * CreditsGraphics.TILE_SIZE,
          CreditsGraphics.TILE_SIZE,
          CreditsGraphics.TILE_SIZE
        );
        tiles.push(surface.subsurface(rect).copy());
      }
    }
    return tiles;
  }

  private static async loadFrames(filename: string): Promise<gameEngine.Surface[]> {
    const surface = await CreditsGraphics.loadSurface(filename);
    const width = surface.get_width();
    const height = surface.get_height();
    const expectedHeight = CreditsGraphics.MON_FRAME_SIZE * CreditsGraphics.FRAMES_PER_SCENE;
    if (width !== CreditsGraphics.MON_FRAME_SIZE || height !== expectedHeight) {
      throw new Error(
        `Credits mon frame sheet ${filename} must be ${CreditsGraphics.MON_FRAME_SIZE}x${expectedHeight}, got ${width}x${height}.`
      );
    }
    const frames: gameEngine.Surface[] = [];
    for (let frameIdx = 0; frameIdx < CreditsGraphics.FRAMES_PER_SCENE; frameIdx += 1) {
      const rect = new gameEngine.Rect(
        0,
        frameIdx * CreditsGraphics.MON_FRAME_SIZE,
        CreditsGraphics.MON_FRAME_SIZE,
        CreditsGraphics.MON_FRAME_SIZE
      );
      frames.push(surface.subsurface(rect).copy());
    }
    return frames;
  }

  private tintSurface(
    surface: gameEngine.Surface,
    palette: [number, number, number][],
    transparentZero = false
  ): gameEngine.Surface {
    if (palette.length !== 4) {
      throw new Error(`Credits palette must have four colours, got ${palette.length}.`);
    }
    const tinted = new gameEngine.Surface(surface.get_width(), surface.get_height());
    for (let y = 0; y < surface.get_height(); y += 1) {
      for (let x = 0; x < surface.get_width(); x += 1) {
        const [r, , , a] = surface.get_at([x, y]);
        const paletteIdx = paletteIndexFromGray(r);
        const target = palette[paletteIdx];
        const alpha = transparentZero && paletteIdx === 0 ? 0 : a;
        tinted.set_at([x, y], [target[0], target[1], target[2], alpha]);
      }
    }
    return tinted;
  }

  private static sceneToMon(sceneIndex: number): string {
    const mapping: Record<number, string> = {
      0: "pichu",
      1: "smoochum",
      2: "ditto",
      3: "igglybuff",
    };
    return mapping[sceneIndex % 4] ?? "pichu";
  }
}

type CreditsLine = {
  tiles: number[][];
  x: number;
  y: number;
  useCopyrightTiles?: boolean;
};

export class CreditsPlayer {
  static readonly SCREEN_WIDTH_TILES = 20;
  static readonly SCREEN_HEIGHT_TILES = 18;
  static readonly TILE_SIZE = 8;
  static readonly LINE_SCROLL_BANDS: Array<[number, number]> = [
    [0x1f, 8],
    [0x87, 8],
  ];
  static readonly SKIP_THRESHOLD = 0x0d;
  static readonly POST_CREDITS_FADE_FRAMES = 32;
  private static readonly ALLOW_SKIP_BIT = 6;
  private static readonly EXIT_BIT = 7;

  private static readonly A_KEYS = new Set(buttonKeys("a"));
  private static readonly B_KEYS = new Set(buttonKeys("b"));

  readonly graphics: CreditsGraphics;
  readonly script: CreditsOp[];
  readonly constants: Record<string, number>;
  private readonly strings: number[][][];
  private readonly stringTable: Record<string, number[][]> = {};

  sceneIndex = 0;
  timer = 0;
  finished = false;
  private pressedKeys = new Set<string | number>();
  private lines: CreditsLine[] = [];
  private scriptIndex = 0;
  private consumedBytes = 0;
  private borderFrameCounter: number | null = null;
  private borderFrameTop: [number, number] | null = null;
  private borderFrameBottom: [number, number] | null = null;
  private borderFramePending: [number, number] | null = null;
  private borderFramePendingBlank = false;
  private borderMonIndex = 0;
  private lyOverride = 0;
  private showTheEnd = false;
  private scriptComplete = false;
  private jumptableIndex: number;
  private readonly jumptableHandlers: Array<() => void>;
  private readonly composedSurface: gameEngine.Surface;
  private readonly scrolledSurface: gameEngine.Surface;
  private readonly borderFrameCompositeCache = new Map<string, gameEngine.Surface>();

  private constructor(
    private readonly ui: ScreenUI,
    private readonly audioEngine: CreditsAudioEngine | null,
    private readonly allowSkip: boolean,
    graphics: CreditsGraphics
  ) {
    this.graphics = graphics;
    this.script = loadCreditsScript();
    this.constants = loadCreditConstantIndices();
    this.strings = loadCreditsStringTiles();
    for (const [name, index] of Object.entries(this.constants)) {
      if (index < 0 || index >= this.strings.length) {
        throw new Error(
          `Credits string index ${index} for constant ${name} is outside the parsed table (size ${this.strings.length}).`
        );
      }
      this.stringTable[name] = this.strings[index];
    }
    this.jumptableIndex = allowSkip ? 1 << CreditsPlayer.ALLOW_SKIP_BIT : 0;
    this.jumptableHandlers = [
      this.stepParse,
      this.creditsNext,
      this.creditsNext,
      this.creditsPrepBgmapUpdate,
      this.creditsUpdateGfxRequestPath,
      this.creditsRequestGfx,
      this.creditsLyOverride,
      this.creditsNext,
      this.creditsNext,
      this.creditsNext,
      this.creditsUpdateGfxRequestPath,
      this.creditsRequestGfx,
      this.creditsLoopBack,
    ];
    this.composedSurface = new gameEngine.Surface(
      CreditsPlayer.SCREEN_WIDTH_TILES * CreditsPlayer.TILE_SIZE,
      CreditsPlayer.SCREEN_HEIGHT_TILES * CreditsPlayer.TILE_SIZE
    );
    this.scrolledSurface = new gameEngine.Surface(
      CreditsPlayer.SCREEN_WIDTH_TILES * CreditsPlayer.TILE_SIZE,
      CreditsPlayer.SCREEN_HEIGHT_TILES * CreditsPlayer.TILE_SIZE
    );
  }

  static async create(
    ui: ScreenUI,
    audioEngine: CreditsAudioEngine | null,
    allowSkip: boolean
  ): Promise<CreditsPlayer> {
    const graphics = await CreditsGraphics.create();
    return new CreditsPlayer(ui, audioEngine, allowSkip, graphics);
  }

  get awaitingExit(): boolean {
    return Boolean(this.jumptableIndex & (1 << CreditsPlayer.EXIT_BIT)) && !this.finished;
  }

  acknowledgeEnd(): void {
    if (this.awaitingExit) {
      this.finished = true;
    }
  }

  reset(): void {
    this.sceneIndex = 0;
    this.timer = 0;
    this.finished = false;
    this.pressedKeys.clear();
    this.lines = [];
    this.scriptIndex = 0;
    this.consumedBytes = 0;
    this.borderFrameCounter = null;
    this.borderFrameTop = null;
    this.borderFrameBottom = null;
    this.borderFramePending = null;
    this.borderFramePendingBlank = false;
    this.borderMonIndex = 0;
    this.lyOverride = 0;
    this.showTheEnd = false;
    this.scriptComplete = false;
    this.jumptableIndex = this.allowSkip ? 1 << CreditsPlayer.ALLOW_SKIP_BIT : 0;
    this.borderFrameCompositeCache.clear();
  }

  handleInput(event: KeyEvent): void {
    if (isKeyDownEvent(event)) {
      const rawKey = event.code || event.key;
      if (rawKey !== null && rawKey !== undefined) {
        const key = String(rawKey);
        this.pressedKeys.add(key);
        if (this.awaitingExit && CreditsPlayer.A_KEYS.has(key)) {
          this.finished = true;
        }
      }
    }
  }

  update(): boolean {
    if (this.finished) {
      this.pressedKeys.clear();
      return true;
    }
    this.handleBButton();
    this.runJumptableStep();
    this.pressedKeys.clear();
    return this.finished;
  }

  draw(): void {
    const screen = this.ui.screen;
    if (!screen) {
      return;
    }
    const [bgPalette, , textPalette] = this.graphics.getPaletteSet(this.sceneIndex);
    const composed = this.composedSurface;
    composed.fill([bgPalette[0][0], bgPalette[0][1], bgPalette[0][2], 255]);
    this.drawMonStrip(composed);
    this.drawTextBackground(composed, textPalette);
    this.drawBorderRows(composed);
    this.drawText(composed, textPalette);
    if (this.showTheEnd) {
      this.drawTheEnd(composed);
    }
    const final = this.applyLineScroll(composed);
    screen.blit(final, [0, 0]);
  }

  private drawTextBackground(
    screen: gameEngine.Surface,
    palette: [number, number, number][]
  ): void {
    const textBg = palette[0];
    gameEngine.draw.rect(
      screen,
      [textBg[0], textBg[1], textBg[2], 255],
      new gameEngine.Rect(
        0,
        5 * CreditsPlayer.TILE_SIZE,
        CreditsPlayer.SCREEN_WIDTH_TILES * CreditsPlayer.TILE_SIZE,
        12 * CreditsPlayer.TILE_SIZE
      )
    );
  }

  private drawBorderRows(screen: gameEngine.Surface): void {
    const tiles = this.graphics.getBorderTiles(
      this.sceneIndex,
      CreditsGraphics.BORDER_PALETTE_INDEX
    );
    this.drawBorderRow(screen, tiles, 4, 4);
    this.drawBorderRow(screen, tiles, 17, 0);
  }

  private drawBorderRow(
    screen: gameEngine.Surface,
    tiles: gameEngine.Surface[],
    row: number,
    baseIndex: number
  ): void {
    if (tiles.length < baseIndex + 4) {
      throw new Error("Credits border tiles are missing required entries.");
    }
    let tileX = 0;
    for (let block = 0; block < CreditsPlayer.SCREEN_WIDTH_TILES / 4; block += 1) {
      for (let offset = 0; offset < 4; offset += 1) {
        const tile = tiles[baseIndex + offset];
        screen.blit(tile, [tileX * CreditsPlayer.TILE_SIZE, row * CreditsPlayer.TILE_SIZE]);
        tileX += 1;
      }
    }
  }

  private drawMonStrip(screen: gameEngine.Surface): void {
    const frame = this.getBorderFrame();
    const frameWidth = frame.get_width();
    const frameHeight = frame.get_height();
    if (frameWidth !== CreditsGraphics.MON_FRAME_SIZE) {
      throw new Error("Credits border frame width is invalid.");
    }
    if (frameHeight !== CreditsGraphics.MON_FRAME_SIZE) {
      throw new Error("Credits border frame height is invalid.");
    }
    for (let x = 0; x < CreditsPlayer.SCREEN_WIDTH_TILES * CreditsPlayer.TILE_SIZE; x += frameWidth) {
      screen.blit(frame, [x, 0]);
    }
  }

  private getBorderFrame(): gameEngine.Surface {
    const cacheKey = `${this.sceneIndex}:${
      this.borderFrameTop ? `${this.borderFrameTop[0]}-${this.borderFrameTop[1]}` : "none"
    }:${
      this.borderFrameBottom ? `${this.borderFrameBottom[0]}-${this.borderFrameBottom[1]}` : "none"
    }`;
    const cached = this.borderFrameCompositeCache.get(cacheKey);
    if (cached) {
      return cached;
    }

    const base = this.graphics.getBlankFrame(this.sceneIndex).copy();
    const frameWidth = base.get_width();
    const frameHeight = base.get_height();
    if (frameHeight % 2 !== 0) {
      throw new Error("Credits border frame height is invalid.");
    }
    const halfHeight = frameHeight / 2;

    if (this.borderFrameTop) {
      const [monIndex, frameIndex] = this.borderFrameTop;
      const frame = this.graphics.getMonFrame(this.sceneIndex, frameIndex, monIndex);
      if (!frame) {
        throw new Error("Credits border frame sprite is unavailable.");
      }
      base.blit(frame, [0, 0], new gameEngine.Rect(0, 0, frameWidth, halfHeight));
    }

    if (this.borderFrameBottom) {
      const [monIndex, frameIndex] = this.borderFrameBottom;
      const frame = this.graphics.getMonFrame(this.sceneIndex, frameIndex, monIndex);
      if (!frame) {
        throw new Error("Credits border frame sprite is unavailable.");
      }
      base.blit(
        frame,
        [0, halfHeight],
        new gameEngine.Rect(0, halfHeight, frameWidth, halfHeight)
      );
    }

    this.borderFrameCompositeCache.set(cacheKey, base);
    return base;
  }

  private drawText(screen: gameEngine.Surface, palette: [number, number, number][]): void {
    const paletteVariants = this.ui.font.paletteVariants;
    if (!paletteVariants) {
      throw new Error("Credits renderer requires a font with palette support.");
    }
    const paletteMap = paletteVariants([palette]);
    let copyrightTiles: gameEngine.Surface[] | null = null;

    for (const block of this.lines) {
      if (block.useCopyrightTiles && !copyrightTiles) {
        copyrightTiles = this.graphics.getCopyrightTiles(this.sceneIndex);
      }
      block.tiles.forEach((tileIds, lineOffset) => {
        let drawX = block.x;
        const drawY = block.y + lineOffset * CreditsPlayer.TILE_SIZE;
        tileIds.forEach((tileId) => {
          let tile: gameEngine.Surface | undefined;
          if (block.useCopyrightTiles) {
            if (tileId < CreditsGraphics.COPYRIGHT_TILE_BASE) {
              throw new Error(
                `Credits copyright tile ${tileId} is below the expected base.`
              );
            }
            const tileIndex = tileId - CreditsGraphics.COPYRIGHT_TILE_BASE;
            if (tileIndex >= CreditsGraphics.COPYRIGHT_TILE_COUNT) {
              throw new Error(
                `Credits copyright tile ${tileId} exceeds the expected range.`
              );
            }
            if (!copyrightTiles) {
              throw new Error("Credits copyright tiles are unavailable.");
            }
            tile = copyrightTiles[tileIndex];
          } else {
            const tilePalettes = paletteMap[tileId];
            if (!tilePalettes || !tilePalettes[0]) {
              throw new Error(`Credits font tile 0x${tileId.toString(16)} is unavailable.`);
            }
            tile = tilePalettes[0];
          }
          screen.blit(tile, [drawX, drawY]);
          drawX += CreditsPlayer.TILE_SIZE;
        });
      });
    }
  }

  private drawTheEnd(screen: gameEngine.Surface): void {
    const graphic = this.graphics.getTheEnd(this.sceneIndex);
    screen.blit(graphic, [6 * CreditsPlayer.TILE_SIZE, 9 * CreditsPlayer.TILE_SIZE]);
  }

  private applyLineScroll(source: gameEngine.Surface): gameEngine.Surface {
    if (this.lyOverride === 0) {
      return source;
    }
    const [width, height] = source.get_size();
    const scrolled = this.scrolledSurface;
    scrolled.blit(source, [0, 0]);

    const scx = this.lyOverride & 0xff;
    let shift = scx < 128 ? scx : scx - 256;
    if (shift === 0) {
      return scrolled;
    }
    if (shift > 0) {
      for (const [start, count] of CreditsPlayer.LINE_SCROLL_BANDS) {
        for (let y = start; y < Math.min(start + count, height); y += 1) {
          scrolled.blit(source, [0, y], new gameEngine.Rect(shift, y, width - shift, 1));
          scrolled.blit(source, [width - shift, y], new gameEngine.Rect(0, y, shift, 1));
        }
      }
      return scrolled;
    }

    shift = -shift;
    for (const [start, count] of CreditsPlayer.LINE_SCROLL_BANDS) {
      for (let y = start; y < Math.min(start + count, height); y += 1) {
        scrolled.blit(source, [shift, y], new gameEngine.Rect(0, y, width - shift, 1));
        scrolled.blit(source, [0, y], new gameEngine.Rect(width - shift, y, shift, 1));
      }
    }
    return scrolled;
  }

  private handleBButton(): void {
    const canSkip = this.canSkip();
    if (!canSkip || this.timer <= 0) {
      return;
    }
    for (const key of this.pressedKeys) {
      if (CreditsPlayer.B_KEYS.has(String(key))) {
        this.timer = Math.max(0, this.timer - 1);
        break;
      }
    }
  }

  private runJumptableStep(): void {
    const step = this.jumptableIndex & 0x0f;
    if (step >= this.jumptableHandlers.length) {
      throw new Error(`Credits jumptable index ${step} is out of range.`);
    }
    this.jumptableHandlers[step]();
  }

  private stepParse = (): void => {
    if (this.jumptableIndex & (1 << CreditsPlayer.EXIT_BIT)) {
      this.creditsNext();
      return;
    }
    if (this.timer > 0) {
      this.timer = Math.max(0, this.timer - 1);
      this.creditsNext();
      return;
    }
    this.clearTextArea();
    while (this.scriptIndex < this.script.length) {
      const op = this.script[this.scriptIndex];
      this.scriptIndex += 1;
      this.consumedBytes += op.byteLength;
      if (op.kind === "string") {
        if (op.lineIndex === null || op.lineIndex === undefined) {
          throw new Error("Credits string opcode missing line index.");
        }
        const tiles = this.resolveText(op.value);
        const y = (6 + op.lineIndex * 2) * CreditsPlayer.TILE_SIZE;
        const isCopyright = this.isCopyrightToken(op.value);
        const xTiles = isCopyright ? 2 : 0;
        this.lines.push({
          tiles,
          x: xTiles * CreditsPlayer.TILE_SIZE,
          y,
          useCopyrightTiles: isCopyright,
        });
        continue;
      }
      if (op.kind === "scene") {
        if (op.value === null || op.value === undefined) {
          throw new Error("Credits scene opcode missing scene index.");
        }
        const paletteCount = this.graphics.paletteSets.length;
        if (!paletteCount) {
          throw new Error("Credits palettes are unavailable for scenes.");
        }
        const sceneValue = Number(op.value);
        this.sceneIndex = sceneValue % paletteCount;
        this.borderFrameCounter = 0;
        continue;
      }
      if (op.kind === "clear") {
        this.borderFrameCounter = null;
        continue;
      }
      if (op.kind === "music") {
        if (this.audioEngine) {
          this.audioEngine.playMusic("MUSIC_NONE", "credits");
          this.audioEngine.skipMusicFrames?.(1);
          this.audioEngine.playMusic("MUSIC_CREDITS", "credits");
        }
        continue;
      }
      if (op.kind === "wait" || op.kind === "wait2") {
        if (op.value === null || op.value === undefined) {
          throw new Error("Credits wait opcode missing duration.");
        }
        this.timer = Math.max(0, Number(op.value));
        this.creditsNext();
        return;
      }
      if (op.kind === "theend") {
        this.showTheEnd = true;
        continue;
      }
      if (op.kind === "end") {
        this.markScriptComplete();
        return;
      }
      throw new Error(`Unsupported credits opcode '${op.kind}'.`);
    }
    this.finished = true;
  };

  private clearTextArea(): void {
    this.lines = [];
    this.showTheEnd = false;
  }

  private creditsNext = (): void => {
    this.jumptableIndex = (this.jumptableIndex + 1) & 0xff;
  };

  private creditsLoopBack = (): void => {
    this.jumptableIndex &= 0xf0;
  };

  private creditsPrepBgmapUpdate = (): void => {
    this.creditsNext();
  };

  private creditsUpdateGfxRequestPath = (): void => {
    this.loadBorderFrame();
    this.creditsNext();
  };

  private creditsRequestGfx = (): void => {
    if (this.borderFramePendingBlank) {
      this.borderFrameBottom = null;
      this.borderFramePendingBlank = false;
    } else if (this.borderFramePending) {
      this.borderFrameBottom = this.borderFramePending;
      this.borderFramePending = null;
    }
    this.creditsNext();
  };

  private creditsLyOverride = (): void => {
    this.lyOverride = (this.lyOverride - 2) & 0xff;
    this.creditsNext();
  };

  private loadBorderFrame(): void {
    if (this.borderFrameCounter === null) {
      this.borderFrameTop = null;
      this.borderFramePending = null;
      this.borderFramePendingBlank = true;
      return;
    }
    this.borderMonIndex = this.sceneIndex;
    const frameIndex = this.borderFrameCounter;
    this.borderFrameTop = [this.borderMonIndex, frameIndex];
    this.borderFramePending = [this.borderMonIndex, frameIndex];
    this.borderFramePendingBlank = false;
    this.borderFrameCounter = (this.borderFrameCounter + 1) % CreditsGraphics.FRAMES_PER_SCENE;
  }

  private markScriptComplete(): void {
    if (this.audioEngine?.fadeOut) {
      this.audioEngine.fadeOut("MUSIC_POST_CREDITS", CreditsPlayer.POST_CREDITS_FADE_FRAMES);
    } else if (this.audioEngine) {
      this.audioEngine.fadeOutMusic(CreditsPlayer.POST_CREDITS_FADE_FRAMES * GB_FRAME_DURATION_MS);
    }
    this.scriptComplete = true;
    this.jumptableIndex |= 1 << CreditsPlayer.EXIT_BIT;
    this.timer = 0;
    this.scriptIndex = this.script.length;
  }

  private resolveText(token: string | number | null | undefined): number[][] {
    if (token === null || token === undefined) {
      throw new Error("Credits string opcode missing token.");
    }
    if (typeof token === "number") {
      if (token < 0 || token >= this.strings.length) {
        throw new Error(`Missing credits string for token index ${token}.`);
      }
      return this.strings[token];
    }
    const key = String(token);
    const resolved = this.stringTable[key];
    if (!resolved) {
      throw new Error(`Missing credits string for token '${key}'.`);
    }
    return resolved;
  }

  private isCopyrightToken(token: string | number | null | undefined): boolean {
    if (token === null || token === undefined) {
      return false;
    }
    if (typeof token === "number") {
      return token === this.constants.COPYRIGHT;
    }
    return String(token) === "COPYRIGHT";
  }

  private canSkip(): boolean {
    if (!(this.jumptableIndex & (1 << CreditsPlayer.ALLOW_SKIP_BIT))) {
      return false;
    }
    return this.consumedBytes >= CreditsPlayer.SKIP_THRESHOLD;
  }
}
