import { beginFlarePlotFrame, finishFlarePlotFrame } from "@pokecrystal/core/ui/flare-plot-renderer";
import { TitleGraphics } from "./title-graphics";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import type { TextSnapshotPayload } from "../text-overlays";
import { buildTitleScreenControlLines } from "../control-lines";
import { mapKeyToButton, mapKeyToDirection } from "@pokecrystal/core/input/controls";
import type { InputEventLike } from "@pokecrystal/core/input/controls";

// ASM reference: engine/menus/intro_menu.asm::TitleScreen* (title screen state and action dispatch).

type ColoredTile = [number, number, number, number][][];
type PriorityMap = Uint8Array[];

export enum TitleScreenState {
  ENTRANCE,
  TIMER,
  MAIN,
  TIMEOUT,
  EXITING,
}

export enum TitleScreenOption {
  MAIN_MENU = "main_menu",
  DELETE_SAVE_DATA = "delete_save_data",
  RESET_CLOCK = "reset_clock",
  RESTART = "restart_intro",
}

class HardwareRegisters {
  scx = 0;
  scy = 0;
  wx = 0;
  wy = 0;
  lcdcPointer: number | null = null;

  setScx(value: number): void {
    this.scx = value & 0xff;
  }
  setScy(value: number): void {
    this.scy = value & 0xff;
  }
  setWx(value: number): void {
    this.wx = value & 0xff;
  }
  setWy(value: number): void {
    this.wy = value & 0xff;
  }
  clearPointer(): void {
    this.lcdcPointer = null;
  }
}

class LineScrollBuffer {
  private readonly values: number[];
  active = false;

  constructor(public readonly height: number) {
    this.values = Array(height).fill(0);
  }

  setActive(active: boolean): void {
    this.active = active;
  }

  setUniform(start: number, count: number, value: number): void {
    if (count <= 0) return;
    value &= 0xff;
    const end = Math.min(start + count, this.height);
    for (let i = start; i < end; i++) {
      this.values[i] = value;
    }
  }

  setInterlaced(start: number, count: number, baseValue: number): void {
    if (count <= 0) return;
    const even = baseValue & 0xff;
    const odd = (~baseValue + 1) & 0xff;
    const end = Math.min(start + count, this.height);
    for (let i = start; i < end; i++) {
      this.values[i] = (i - start) % 2 === 0 ? even : odd;
    }
  }

  get(idx: number): number | null {
    if (!this.active || idx < 0 || idx >= this.height) {
      return null;
    }
    return this.values[idx];
  }
}

export class TitleScreen {
  static readonly SCREEN_WIDTH_TILES = 20;
  static readonly SCREEN_HEIGHT_TILES = 18;
  static readonly TILE_SIZE = 8;
  private static readonly MUSIC_FADE_STEP_FRAMES = 8;
  private static readonly LOGO_START_TILE = 0x80;
  private static readonly LOGO_ROWS = 7;
  private static readonly LOGO_COLS = 20;
  private static readonly VERSION_TEXT_START_TILE = 0x0c;
  private static readonly VERSION_TEXT_START_COLUMN = 3;
  private static readonly VERSION_TEXT_ROW = 0;
  private static readonly VERSION_TEXT_COLUMNS = 13;
  // Visual layout override: the ASM title screen anchors Suicune at `hlcoord 6, 12`,
  // but this TypeScript title composition keeps the sprite slightly higher so it
  // remains visually connected to the CRYSTAL VERSION wordmark in the current web layout.
  private static readonly SUICUNE_START_Y_TILE = 11;
  static readonly SCREEN_WIDTH_PX =
    TitleScreen.SCREEN_WIDTH_TILES * TitleScreen.TILE_SIZE;
  static readonly SCREEN_HEIGHT_PX =
    TitleScreen.SCREEN_HEIGHT_TILES * TitleScreen.TILE_SIZE;
  static readonly BG_MAP_WIDTH_TILES = 32;
  static readonly BG_MAP_HEIGHT_TILES = 32;
  static readonly BG_MAP_WIDTH_PX =
    TitleScreen.BG_MAP_WIDTH_TILES * TitleScreen.TILE_SIZE;
  static readonly BG_MAP_HEIGHT_PX =
    TitleScreen.BG_MAP_HEIGHT_TILES * TitleScreen.TILE_SIZE;

  private readonly graphics: TitleGraphics;
  private readonly registers = new HardwareRegisters();
  private state = TitleScreenState.ENTRANCE;
  private framesSinceEntranceStart = 0;
  private titleTimeoutFrames: number;
  private titleTimer = 0;
  private musicFadeFrames: number;
  private fadeFramesRemaining = 0;
  private pendingAction: TitleScreenOption | null = null;

  private suicuneFrame = 0;
  private suicuneAnimationTimer = 0;
  private crystalSprites: [number, number, number][] = [];
  private readonly backgroundCanvas: HTMLCanvasElement;
  private readonly windowCanvas: HTMLCanvasElement;
  private readonly scrolledBackgroundCanvas: HTMLCanvasElement;
  private readonly crystalCanvas: HTMLCanvasElement;
  private readonly backgroundContext: CanvasRenderingContext2D | null;
  private readonly windowContext: CanvasRenderingContext2D | null;
  private readonly scrolledBackgroundContext: CanvasRenderingContext2D | null;
  private readonly crystalContext: CanvasRenderingContext2D | null;
  private backgroundLayerDirty = true;
  private windowLayerDirty = true;
  private scrolledBackgroundLayerDirty = true;
  private crystalLayerDirty = true;
  private renderedWindowWy: number | null = null;
  private renderedWindowWx: number | null = null;
  private renderedScrollKey: string | null = null;

  private readonly lineScrollBuffer = new LineScrollBuffer(160);
  private readonly backgroundPriorityMap: PriorityMap = Array.from(
    { length: TitleScreen.SCREEN_HEIGHT_PX },
    () => new Uint8Array(TitleScreen.SCREEN_WIDTH_PX)
  );
  private backgroundPriorityMapDirty = true;

  private readonly bgPaletteMap: number[][] = [];
  private readonly bg1PaletteMap: number[][] = [];

  private keysHeld = new Set<string>();
  private keysPressedFrame = new Set<string>();
  private clockResetTrigger = false;
  private static readonly STATE_NAMES: Record<TitleScreenState, string> = {
    [TitleScreenState.ENTRANCE]: "entrance",
    [TitleScreenState.TIMER]: "timer",
    [TitleScreenState.MAIN]: "main",
    [TitleScreenState.TIMEOUT]: "timeout",
    [TitleScreenState.EXITING]: "exiting",
  };


  private constructor(
    graphics: TitleGraphics,
    private readonly audioEngine: AudioEngine
  ) {
    this.graphics = graphics;
    this.titleTimeoutFrames = 73 * 60 + 36;
    this.musicFadeFrames = 8;
    const backgroundSurface = this._createOffscreenCanvas();
    const windowSurface = this._createOffscreenCanvas();
    const scrolledBackgroundSurface = this._createOffscreenCanvas();
    const crystalSurface = this._createOffscreenCanvas();
    this.backgroundCanvas = backgroundSurface.canvas;
    this.backgroundContext = backgroundSurface.context;
    this.windowCanvas = windowSurface.canvas;
    this.windowContext = windowSurface.context;
    this.scrolledBackgroundCanvas = scrolledBackgroundSurface.canvas;
    this.scrolledBackgroundContext = scrolledBackgroundSurface.context;
    this.crystalCanvas = crystalSurface.canvas;
    this.crystalContext = crystalSurface.context;

    for (let y = 0; y < TitleScreen.SCREEN_HEIGHT_TILES; y++) {
        this.bgPaletteMap[y] = Array(TitleScreen.SCREEN_WIDTH_TILES).fill(0);
        this.bg1PaletteMap[y] = Array(TitleScreen.SCREEN_WIDTH_TILES).fill(0);
      }

    this._initializeTitleScreen();
  }

  static async create(audioEngine: AudioEngine): Promise<TitleScreen> {
    const graphics = await TitleGraphics.create();
    return new TitleScreen(graphics, audioEngine);
  }

  startFromGameStart(): void {
    this._clearSprites();
    this._initializeSuicune();
    this._initializeCrystalBackground();
    this._prepareEntranceAnimation();
    this._invalidateRenderedLayers();
    this.pendingAction = null;
    this.clockResetTrigger = false;
    this.keysHeld.clear();
    this.keysPressedFrame.clear();
    this.audioEngine.channelsOff();
    this.audioEngine.playSound("SFX_TITLE_SCREEN_ENTRANCE");
  }

  private _initializeTitleScreen(): void {
    this._setupPalettes();
    this._clearSprites();
    this._initializeSuicune();
    this._initializeCrystalBackground();
    this._prepareEntranceAnimation();
    this._invalidateRenderedLayers();
  }

  private _createOffscreenCanvas(): {
    canvas: HTMLCanvasElement;
    context: CanvasRenderingContext2D | null;
  } {
    if (typeof document === "undefined") {
      const fallbackCanvas = {
        width: 0,
        height: 0,
        getContext: () => null,
      } as unknown as HTMLCanvasElement;
      return {
        canvas: fallbackCanvas,
        context: null,
      };
    }
    const canvas = document.createElement("canvas");
    canvas.width = TitleScreen.SCREEN_WIDTH_PX;
    canvas.height = TitleScreen.SCREEN_HEIGHT_PX;
    const context = canvas.getContext("2d");
    return { canvas, context };
  }

  private _setupPalettes(): void {
    for (let x = 0; x < TitleScreen.SCREEN_WIDTH_TILES; x++) {
      this.bg1PaletteMap[0][x] = 7;
    }

    const fillRow = (row: number, palette: number) => {
      for (let x = 0; x < TitleScreen.SCREEN_WIDTH_TILES; x++) {
        this.bgPaletteMap[row][x] = palette;
      }
    };

    fillRow(3, 2);
    fillRow(4, 2);
    fillRow(5, 3);
    fillRow(6, 4);
    fillRow(7, 5);
    fillRow(8, 6);
    fillRow(9, 6);

    for (let x = 5; x < 5 + 11; x++) {
      this.bgPaletteMap[9][x] = 1;
    }
  }

  private _clearSprites(): void {
    this.crystalSprites = [];
  }

  private _initializeSuicune(): void {
    this.suicuneFrame = 0;
    this._invalidateBackgroundPriorityMap();
    this._invalidateBackgroundLayer();
  }

  private _initializeCrystalBackground(): void {
    this.crystalSprites = [];
    let y = -0x22 & 0xff;
    let tileId = 0;
    for (let i = 0; i < 5; i++) {
      let x = 0x40;
      for (let j = 0; j < 6; j++) {
        this.crystalSprites.push([x, y, tileId]);
        x = (x + 8) & 0xff;
        tileId = (tileId + 2) & 0xff;
      }
      y = (y + 0x10) & 0xff;
    }
  }

  private _prepareEntranceAnimation(): void {
    this.state = TitleScreenState.ENTRANCE;
    this.registers.setScx(112);
    this.registers.setScy(8);
    this.registers.setWx(7);
    this.registers.setWy(-112 & 0xff);
    this.registers.lcdcPointer = 0x43;
    this._invalidateWindowLayer();
    this._invalidateBackgroundLayer();

    this.lineScrollBuffer.setActive(true);
    this.lineScrollBuffer.setInterlaced(0, 80, this.registers.scx);
    this.lineScrollBuffer.setUniform(80, 80, 0);
    this._invalidateBackgroundPriorityMap();

    this.framesSinceEntranceStart = 0;
    this.titleTimer = 0;
    this.fadeFramesRemaining = 0;
    this.pendingAction = null;
  }

  update(): void {
    switch (this.state) {
      case TitleScreenState.ENTRANCE:
        this._updateEntrance();
        break;
      case TitleScreenState.TIMER:
        this._updateTimer();
        break;
      case TitleScreenState.MAIN:
        this._updateMain();
        break;
      case TitleScreenState.TIMEOUT:
        this._updateTimeout();
        break;
      case TitleScreenState.EXITING:
        this._animateCrystal();
        break;
    }
    this._updateSuicuneAnimation();
  }

  private _updateEntrance(): void {
    const scx = this.registers.scx;
    if (scx !== 0) {
      this._animateCrystal();
    }

    if (scx === 0) {
      this._finishEntrance();
      return;
    }

    const newScx = (scx - 4) & 0xff;
    this.registers.setScx(newScx);
    this.lineScrollBuffer.setInterlaced(0, 80, newScx);
    this._invalidateScrolledBackgroundLayer();
    this._invalidateBackgroundPriorityMap();
    this.framesSinceEntranceStart++;
  }

  private _finishEntrance(): void {
    if (this.state !== TitleScreenState.ENTRANCE) return;
    this.audioEngine.playMusic("MUSIC_TITLE", "title");
    this._startTimerState();
  }

  private _startTimerState(): void {
    this.state = TitleScreenState.TIMER;
    this.lineScrollBuffer.setActive(false);
    this._invalidateScrolledBackgroundLayer();
    this.registers.clearPointer();
    this.registers.setWy(0x88);
    this._invalidateWindowLayer();
    this.titleTimer = 0;
    this._invalidateBackgroundPriorityMap();
  }

  private _updateTimer(): void {
    this._startMainState();
  }

  private _startMainState(): void {
    this.state = TitleScreenState.MAIN;
    this.titleTimer = this.titleTimeoutFrames;
  }

  private _updateMain(): void {
    if (this.titleTimer <= 0) {
      this._beginTimeoutExit();
      this.keysPressedFrame.clear();
      return;
    }
    this._evaluateMainInputs();
    this.titleTimer--;
    this._animateCrystal();
    this.keysPressedFrame.clear();
  }

  private _beginTimeoutExit(): void {
    this.state = TitleScreenState.TIMEOUT;
    this.fadeFramesRemaining = this.musicFadeFrames * TitleScreen.MUSIC_FADE_STEP_FRAMES;
    this.audioEngine.fadeOutMusic(
      (this.musicFadeFrames * TitleScreen.MUSIC_FADE_STEP_FRAMES * 1000) / 60
    );
  }

  private _updateTimeout(): void {
    if (this.fadeFramesRemaining > 0) {
      this.fadeFramesRemaining--;
      return;
    }
    this.audioEngine.stopMusic();
    this._queueAction(TitleScreenOption.RESTART);
  }

  private _queueAction(option: TitleScreenOption): void {
    if (this.pendingAction) return;
    this.pendingAction = option;
    this.state = TitleScreenState.EXITING;
  }

  private _isHeld(keys: string[]): boolean {
    return keys.some((key) => this.keysHeld.has(key));
  }

  private _isComboPressed(keys: string[][]): boolean {
    return keys.every((keyGroup) =>
      keyGroup.some((key) => this.keysPressedFrame.has(key))
    );
  }

  private _evaluateMainInputs(): void {
    if (this.state !== TitleScreenState.MAIN || this.pendingAction) {
      return;
    }

    if (this._isComboPressed([["ArrowUp"], ["b"], ["Select"]])) {
      this._queueAction(TitleScreenOption.DELETE_SAVE_DATA);
      return;
    }

    if (!this.clockResetTrigger) {
      if (this._isComboPressed([["ArrowDown"], ["b"], ["Select"]])) {
        this.clockResetTrigger = true;
      }
    } else if (!this._isHeld(["Select"])) {
      this.clockResetTrigger = false;
      if (this._isHeld(["ArrowLeft"]) && this._isHeld(["ArrowUp"])) {
        this._queueAction(TitleScreenOption.RESET_CLOCK);
        return;
      }
    }

    if (this.keysPressedFrame.has("Enter") || this.keysPressedFrame.has("a")) {
      this._queueAction(TitleScreenOption.MAIN_MENU);
    }
  }

  private _animateCrystal(): void {
    if (!this.crystalSprites.length || this.crystalSprites[0][1] === 22) {
      return;
    }
    for (let i = 0; i < this.crystalSprites.length; i++) {
      const [x, y, tileId] = this.crystalSprites[i];
      this.crystalSprites[i] = [x, (y + 2) & 0xff, tileId];
    }
    this._invalidateCrystalLayer();
  }

  private _getSuicuneTileForFrame(frame: number): number {
    const frameTiles = [0x80, 0x88, 0x00, 0x08];
    return frameTiles[frame % 4];
  }

  private _updateSuicuneAnimation(): void {
    this.suicuneAnimationTimer++;
    if (this.suicuneAnimationTimer >= 8) {
      this.suicuneFrame = (this.suicuneFrame + 1) % 4;
      this.suicuneAnimationTimer = 0;
      this._invalidateBackgroundLayer();
      this._invalidateBackgroundPriorityMap();
    }
  }

  private _invalidateBackgroundPriorityMap(): void {
    this.backgroundPriorityMapDirty = true;
    this._invalidateCrystalLayer();
  }

  private _invalidateBackgroundLayer(): void {
    this.backgroundLayerDirty = true;
    this._invalidateScrolledBackgroundLayer();
  }

  private _invalidateWindowLayer(): void {
    this.windowLayerDirty = true;
    this.renderedWindowWy = null;
    this.renderedWindowWx = null;
  }

  private _invalidateRenderedLayers(): void {
    this._invalidateBackgroundLayer();
    this._invalidateWindowLayer();
    this._invalidateCrystalLayer();
  }

  private _invalidateScrolledBackgroundLayer(): void {
    this.scrolledBackgroundLayerDirty = true;
    this.renderedScrollKey = null;
  }

  private _invalidateCrystalLayer(): void {
    this.crystalLayerDirty = true;
  }

  private _drawTile(
    ctx: CanvasRenderingContext2D,
    tile: ColoredTile,
    x: number,
    y: number
  ) {
    for (let row = 0; row < TitleScreen.TILE_SIZE; row++) {
      for (let col = 0; col < TitleScreen.TILE_SIZE; col++) {
        const [r, g, b, a] = tile[row][col];
        if (a > 0) {
          ctx.fillStyle = `rgba(${r},${g},${b},${a / 255})`;
          ctx.fillRect(x + col, y + row, 1, 1);
        }
      }
    }
  }

  private _renderBackground(ctx: CanvasRenderingContext2D) {
    ctx.clearRect(0, 0, TitleScreen.SCREEN_WIDTH_PX, TitleScreen.SCREEN_HEIGHT_PX);

    const suicuneStartTile = this._getSuicuneTileForFrame(this.suicuneFrame);
    for (let row = 0; row < 6; row++) {
        for (let col = 0; col < 8; col++) {
            const tileId = (suicuneStartTile + row * 16 + col) & 0xFF;
            const tile = this.graphics.getTile("suicune", tileId, 0);
            this._drawTile(
              ctx,
              tile,
              (6 + col) * TitleScreen.TILE_SIZE,
              (TitleScreen.SUICUNE_START_Y_TILE + row) * TitleScreen.TILE_SIZE
            );
        }
    }

    for (let row = 0; row < TitleScreen.LOGO_ROWS; row++) {
        for (let col = 0; col < TitleScreen.LOGO_COLS; col++) {
            const tileId =
              TitleScreen.LOGO_START_TILE + row * TitleScreen.LOGO_COLS + col;
            const palette = this.bgPaletteMap[3 + row][col];
            const tile = this.graphics.getTile("logo", tileId, palette);
            this._drawTile(
              ctx,
              tile,
              col * TitleScreen.TILE_SIZE,
              (3 + row) * TitleScreen.TILE_SIZE
            );
        }
    }
  }

  private _renderWindow(ctx: CanvasRenderingContext2D) {
    ctx.clearRect(0, 0, TitleScreen.SCREEN_WIDTH_PX, TitleScreen.SCREEN_HEIGHT_PX);

    const wy = this.registers.wy & 0xFF;
    if (wy >= TitleScreen.SCREEN_HEIGHT_PX) {
        return;
    }

    const wx_offset = (this.registers.wx - 7) & 0xFF;
    if (wx_offset >= TitleScreen.SCREEN_WIDTH_PX) {
        return;
    }

    for (let row = 0; row < 1; row++) {
        for (let col = 0; col < TitleScreen.VERSION_TEXT_COLUMNS; col++) {
            const tileId =
              TitleScreen.VERSION_TEXT_START_TILE +
              row * TitleScreen.VERSION_TEXT_COLUMNS +
              col;
            const palette =
              this.bg1PaletteMap[row + TitleScreen.VERSION_TEXT_ROW][col + TitleScreen.VERSION_TEXT_START_COLUMN];
            const tile = this.graphics.getTile("logo", tileId, palette);
            this._drawTile(
              ctx,
              tile,
              wx_offset + (col + TitleScreen.VERSION_TEXT_START_COLUMN) * TitleScreen.TILE_SIZE,
              wy + row * TitleScreen.TILE_SIZE
            );
        }
    }
  }

  private _drawCrystalSprites(ctx: CanvasRenderingContext2D): void {
    const priorityMap = this._getBackgroundPriorityMap();
    for (const [x, y, tileId] of this.crystalSprites) {
        const baseTile = tileId & 0xfe;
        const topTile = this.graphics.getTile("crystal", baseTile, 0);
        const topIndices = this.graphics.getTileIndices("crystal", baseTile);
        const bottomTile = this.graphics.getTile("crystal", (baseTile + 1) & 0xff, 0);
        const bottomIndices = this.graphics.getTileIndices("crystal", (baseTile + 1) & 0xff);

        const drawX = x - 8;
        const drawY = y - 16;

        this._drawCrystalTileWithPriority(
          ctx,
          topTile,
          topIndices,
          drawX,
          drawY,
          priorityMap
        );
        this._drawCrystalTileWithPriority(
          ctx,
          bottomTile,
          bottomIndices,
          drawX,
          drawY + 8,
          priorityMap
        );
    }
  }

  private _drawCrystalTileWithPriority(
    ctx: CanvasRenderingContext2D,
    tile: ColoredTile,
    tileIndices: number[][],
    x: number,
    y: number,
    priorityMap: PriorityMap
  ): void {
    for (let row = 0; row < TitleScreen.TILE_SIZE; row++) {
      for (let col = 0; col < TitleScreen.TILE_SIZE; col++) {
        const drawX = x + col;
        const drawY = y + row;
        if (
          drawX < 0 ||
          drawY < 0 ||
          drawX >= TitleScreen.SCREEN_WIDTH_PX ||
          drawY >= TitleScreen.SCREEN_HEIGHT_PX
        ) {
          continue;
        }

        // The crystal sprites use OAM_PRIO in ASM, so they only appear when BG/WIN color is index 0.
        if ((tileIndices[row]?.[col] ?? 0) === 0 || (priorityMap[drawY]?.[drawX] ?? 0) !== 0) {
          continue;
        }

        const [r, g, b, a] = tile[row][col];
        if (a > 0) {
          ctx.fillStyle = `rgba(${r},${g},${b},${a / 255})`;
          ctx.fillRect(drawX, drawY, 1, 1);
        }
      }
    }
  }

  private _buildBackgroundPriorityMap(): PriorityMap {
    const map = this.backgroundPriorityMap;
    for (let y = 0; y < TitleScreen.SCREEN_HEIGHT_PX; y++) {
      map[y].fill(0);
    }
    const stampTile = (
      tileIndices: number[][],
      originX: number,
      originY: number,
      applyBackgroundScroll = false
    ) => {
      for (let row = 0; row < TitleScreen.TILE_SIZE; row++) {
        for (let col = 0; col < TitleScreen.TILE_SIZE; col++) {
          const sourceX = originX + col;
          const drawY = originY + row;
          if (
            sourceX < 0 ||
            drawY < 0 ||
            sourceX >= TitleScreen.SCREEN_WIDTH_PX ||
            drawY >= TitleScreen.SCREEN_HEIGHT_PX
          ) {
            continue;
          }
          const scroll = applyBackgroundScroll
            ? this.lineScrollBuffer.get(drawY) ?? this.registers.scx
            : 0;
          const drawX = applyBackgroundScroll
            ? (sourceX - scroll + TitleScreen.SCREEN_WIDTH_PX) % TitleScreen.SCREEN_WIDTH_PX
            : sourceX;
          map[drawY][drawX] = tileIndices[row]?.[col] ?? 0;
        }
      }
    };

    const suicuneStartTile = this._getSuicuneTileForFrame(this.suicuneFrame);
    for (let row = 0; row < 6; row++) {
      for (let col = 0; col < 8; col++) {
        const tileId = (suicuneStartTile + row * 16 + col) & 0xff;
        stampTile(
          this.graphics.getTileIndices("suicune", tileId),
          (6 + col) * TitleScreen.TILE_SIZE,
          (TitleScreen.SUICUNE_START_Y_TILE + row) * TitleScreen.TILE_SIZE,
          true
        );
      }
    }

    for (let row = 0; row < TitleScreen.LOGO_ROWS; row++) {
      for (let col = 0; col < TitleScreen.LOGO_COLS; col++) {
        const tileId = TitleScreen.LOGO_START_TILE + row * TitleScreen.LOGO_COLS + col;
        stampTile(
          this.graphics.getTileIndices("logo", tileId),
          col * TitleScreen.TILE_SIZE,
          (3 + row) * TitleScreen.TILE_SIZE,
          true
        );
      }
    }

    const wy = this.registers.wy & 0xff;
    const wxOffset = (this.registers.wx - 7) & 0xff;
    if (wy < TitleScreen.SCREEN_HEIGHT_PX && wxOffset < TitleScreen.SCREEN_WIDTH_PX) {
      for (let row = 0; row < 1; row++) {
        for (let col = 0; col < TitleScreen.VERSION_TEXT_COLUMNS; col++) {
          const tileId =
            TitleScreen.VERSION_TEXT_START_TILE +
            row * TitleScreen.VERSION_TEXT_COLUMNS +
            col;
          stampTile(
            this.graphics.getTileIndices("logo", tileId),
            wxOffset + (col + TitleScreen.VERSION_TEXT_START_COLUMN) * TitleScreen.TILE_SIZE,
            wy + row * TitleScreen.TILE_SIZE
          );
        }
      }
    }

    return map;
  }

  private _getBackgroundPriorityMap(): PriorityMap {
    if (this.backgroundPriorityMapDirty) {
      this._buildBackgroundPriorityMap();
      this.backgroundPriorityMapDirty = false;
    }
    return this.backgroundPriorityMap;
  }

  private _ensureBackgroundLayerRendered(): void {
    if (this.backgroundContext === null || !this.backgroundLayerDirty) {
      return;
    }
    this._renderBackground(this.backgroundContext);
    this.backgroundLayerDirty = false;
    this._invalidateScrolledBackgroundLayer();
  }

  private _ensureWindowLayerRendered(): void {
    if (this.windowContext === null) {
      return;
    }
    const wy = this.registers.wy & 0xff;
    const wx = this.registers.wx & 0xff;
    if (!this.windowLayerDirty && this.renderedWindowWy === wy && this.renderedWindowWx === wx) {
      return;
    }
    this._renderWindow(this.windowContext);
    this.windowLayerDirty = false;
    this.renderedWindowWy = wy;
    this.renderedWindowWx = wx;
    this._invalidateCrystalLayer();
  }

  private _buildScrollKey(): string {
    if (!this.lineScrollBuffer.active) {
      return `scx:${this.registers.scx & 0xff}`;
    }
    const values = (this.lineScrollBuffer as unknown as { values?: number[] }).values ?? [];
    return `lines:${values.join(",")}`;
  }

  private _ensureScrolledBackgroundLayerRendered(): void {
    if (this.scrolledBackgroundContext === null || this.backgroundContext === null) {
      return;
    }
    this._ensureBackgroundLayerRendered();
    const scrollKey = this._buildScrollKey();
    if (!this.scrolledBackgroundLayerDirty && this.renderedScrollKey === scrollKey) {
      return;
    }

    const ctx = this.scrolledBackgroundContext;
    ctx.clearRect(0, 0, TitleScreen.SCREEN_WIDTH_PX, TitleScreen.SCREEN_HEIGHT_PX);
    for (let y = 0; y < TitleScreen.SCREEN_HEIGHT_PX; y++) {
      const scroll = this.lineScrollBuffer.get(y) ?? this.registers.scx;
      const sourceY = y;
      ctx.drawImage(
        this.backgroundCanvas,
        scroll,
        sourceY,
        TitleScreen.SCREEN_WIDTH_PX - scroll,
        1,
        0,
        y,
        TitleScreen.SCREEN_WIDTH_PX - scroll,
        1
      );
      if (scroll > 0) {
        ctx.drawImage(
          this.backgroundCanvas,
          0,
          sourceY,
          scroll,
          1,
          TitleScreen.SCREEN_WIDTH_PX - scroll,
          y,
          scroll,
          1
        );
      }
    }
    this.scrolledBackgroundLayerDirty = false;
    this.renderedScrollKey = scrollKey;
  }

  private _ensureCrystalLayerRendered(): void {
    if (this.crystalContext === null) {
      return;
    }
    if (!this.crystalLayerDirty) {
      return;
    }
    this.crystalContext.clearRect(0, 0, TitleScreen.SCREEN_WIDTH_PX, TitleScreen.SCREEN_HEIGHT_PX);
    this._drawCrystalSprites(this.crystalContext);
    this.crystalLayerDirty = false;
  }


  draw(ctx: CanvasRenderingContext2D): void {
    const flareStart = beginFlarePlotFrame();
    ctx.save();
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.globalAlpha = 1;
    ctx.globalCompositeOperation = "source-over";

    ctx.fillStyle = "black";
    ctx.fillRect(
      0,
      0,
      TitleScreen.SCREEN_WIDTH_PX,
      TitleScreen.SCREEN_HEIGHT_PX
    );

    if (this.scrolledBackgroundContext !== null) {
      this._ensureScrolledBackgroundLayerRendered();
      ctx.drawImage(this.scrolledBackgroundCanvas, 0, 0);
    }

    if (this.windowContext !== null) {
      this._ensureWindowLayerRendered();
      ctx.drawImage(this.windowCanvas, 0, 0);
    }

    if (this.crystalContext !== null) {
      this._ensureCrystalLayerRendered();
      ctx.drawImage(this.crystalCanvas, 0, 0);
    } else {
      this._drawCrystalSprites(ctx);
    }

    ctx.restore();
    finishFlarePlotFrame(flareStart, "title", ctx, 0, 0, 100, 30);
  }

  handleInput(event: KeyboardEvent | InputEventLike, isKeyDown: boolean): void {
    const key = this._normalizeInputKey(event);
    if (!key) {
      return;
    }
    if (isKeyDown) {
      this.keysHeld.add(key);
      this.keysPressedFrame.add(key);
    } else {
      this.keysHeld.delete(key);
    }
  }

  private _normalizeInputKey(event: KeyboardEvent | InputEventLike): string {
    const raw = "code" in event && event.code ? event.code : event.key;
    const buttonFromEvent = "button" in event ? mapKeyToButton(event.button ?? null) : null;
    const button = buttonFromEvent ?? mapKeyToButton(raw ?? null);
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
    const direction = mapKeyToDirection(raw ?? null);
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
    return typeof event.key === "string" ? event.key : "";
  }

  popAction(): TitleScreenOption | null {
    const action = this.pendingAction;
    this.pendingAction = null;
    return action;
  }

  getTextSnapshot(): TextSnapshotPayload {
    const stateName = TitleScreen.STATE_NAMES[this.state] ?? "unknown";
    const controlLines =
      this.state === TitleScreenState.MAIN
        ? buildTitleScreenControlLines("main")
        : this.state === TitleScreenState.TIMEOUT
          ? buildTitleScreenControlLines("timeout")
          : buildTitleScreenControlLines("entrance");
    return {
      viewportLines: [
        "POKEMON CRYSTAL",
        this.state === TitleScreenState.MAIN ? "PRESS START" : "TITLE SCREEN",
      ],
      infoLines: [
        `STATE: ${stateName}`,
        `TIMER: ${this.titleTimer}`,
        `SCX: ${this.registers.scx}`,
        `WY: ${this.registers.wy & 0xff}`,
        `PENDING ACTION: ${this.pendingAction ?? "none"}`,
        ...controlLines,
      ],
      viewportTitle: "Title",
      infoTitle: "Title",
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
    };
  }
}
