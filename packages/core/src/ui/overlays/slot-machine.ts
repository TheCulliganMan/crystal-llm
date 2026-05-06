import fs from "fs";
import { MAX_COINS } from "@pokecrystal/core/core/constants";
import { gbc5To8, type RGB } from "@pokecrystal/core/core/gbc-colors";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import type { GameState } from "@pokecrystal/core/core/state";
import { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import {
  REEL_LENGTH,
  REEL_TILEMAPS,
  SlotMachine,
  SlotMachineMode,
  SlotSymbol,
  type SlotMachineResult,
} from "@pokecrystal/core/engine/games/slots";
import { GameButton, isButtonEvent, isKeyDownEvent } from "@pokecrystal/core/input/buttons";
import { nextFrame } from "@pokecrystal/core/ui/async-loop";
import { gameEngine, type GameEngineEventQueue } from "@pokecrystal/core/ui/game-engine";
import type { Surface } from "@pokecrystal/core/ui/surface";
import { renderTextSnapshot } from "@pokecrystal/core/ui/text-overlays";

type SlotMachineUI = {
  eventQueue?: GameEngineEventQueue;
  screen?: Pick<Surface, "fill" | "width" | "height">;
  font?: {
    render_text?: (
      text: string,
      x: number,
      y: number,
      surface: Surface,
      options?: { uppercase?: boolean; color?: [number, number, number] },
    ) => void;
    renderText?: (
      text: string,
      x: number,
      y: number,
      surface: Surface,
      options?: { uppercase?: boolean; color?: [number, number, number] },
    ) => void;
  };
  update?: () => void;
};

type AudioEngineLike = {
  play_sound?: (name: string) => void;
  playSound?: (name: string) => void;
};

type ReelWindow = [SlotSymbol, SlotSymbol, SlotSymbol];
type ReelWindows = [ReelWindow, ReelWindow, ReelWindow];

type PendingSlotAnimation = {
  frame: number;
  baseOffsets: [number, number, number];
  targetOffsets: [number, number, number];
  result: SlotMachineResult;
  nextCoins: number;
  message: string;
  stopFrames: readonly [number, number, number];
};

type SlotMachineOverlayOptions = {
  bet?: number;
  mode?: SlotMachineMode;
  frameAwaiter?: () => Promise<void>;
  animation?: {
    stopFrames?: readonly [number, number, number];
  };
};

export type SlotMachineOverlayOutcome = {
  played: boolean;
  bet: number;
  payout: number;
  matched_symbol: keyof typeof SlotSymbol | null;
  winning_lines: string[];
  coins: number;
};

const SYMBOL_LABELS: Record<SlotSymbol, string> = {
  [SlotSymbol.SEVEN]: "7",
  [SlotSymbol.POKEBALL]: "BALL",
  [SlotSymbol.CHERRY]: "CHERRY",
  [SlotSymbol.PIKACHU]: "PIKA",
  [SlotSymbol.SQUIRTLE]: "SQUIRT",
  [SlotSymbol.STARYU]: "STARYU",
};

const TILE_SIZE = 8;
const SLOTS_TILEMAP_WIDTH = 20;
const SLOTS_TILEMAP_HEIGHT = 12;
const SLOTS_TILEMAP_LENGTH = SLOTS_TILEMAP_WIDTH * SLOTS_TILEMAP_HEIGHT;
const SLOTS_VTILES2_OVERLAY_START_TILE = 0x25;
const SLOT_COIN_COUNT_TILE_X = 5;
const SLOT_PAYOUT_TILE_X = 11;
const SLOT_COUNTER_TILE_Y = 1;
const SLOT_PROMPT_TILE_X = 1;
const SLOT_PROMPT_TILE_Y = 15;
const SLOT_REEL_X_TILES = [5, 9, 13] as const;
const SLOT_REEL_Y_TILES = [4, 6, 8] as const;
const SLOT_ICON_TILE_SIZE = 8;
const SLOT_ICON_SPRITE_TILES = 2;
const SLOT_ICON_SURFACE_SIZE = SLOT_ICON_TILE_SIZE * SLOT_ICON_SPRITE_TILES;
const SLOT_ICON_TILE_STRIDE = 4;
const SLOT_REEL_STOP_FRAMES: readonly [number, number, number] = [24, 36, 48];

const resolveSlotsUiTilesPath = () => getAssetPath("gfx", "slots", "slots_1.png");
const resolveSlotsSymbolTilesPath = () => getAssetPath("gfx", "slots", "slots_2.png");
const resolveSlotsTilemapPath = () => getAssetPath("gfx", "slots", "slots.tilemap");
const resolveSlotsPalettePath = () => getAssetPath("gfx", "slots", "slots.pal");

let slotsUiSheetCache: Surface | null = null;
let slotsSymbolSheetCache: Surface | null = null;
let slotsTilemapCache: Uint8Array | null = null;
let slotsPaletteCache: RGB[][] | null = null;
const slotsPaletteSheetCache = new WeakMap<Surface, Map<string, Surface>>();
const slotIconCache = new Map<SlotSymbol, Surface>();

const playSound = (audio: AudioEngineLike | null | undefined, name: string): void => {
  if (audio?.play_sound) {
    audio.play_sound(name);
    return;
  }
  audio?.playSound?.(name);
};

const clampCoins = (coins: number): number =>
  Math.max(0, Math.min(MAX_COINS, Math.trunc(coins)));

const renderPixelText = (
  ui: SlotMachineUI,
  text: string,
  x: number,
  y: number,
  options: { uppercase?: boolean; color?: [number, number, number] } = {},
): void => {
  const screen = ui.screen as Surface | undefined;
  if (!screen) {
    return;
  }
  const render = ui.font?.render_text ?? ui.font?.renderText;
  if (typeof render !== "function") {
    return;
  }
  render.call(ui.font, text, x, y, screen, { uppercase: options.uppercase ?? true, ...options });
};

const loadSlotsUiSheet = (): Surface => {
  const cached = slotsUiSheetCache;
  if (cached) {
    return cached;
  }
  const path = resolveSlotsUiTilesPath();
  const loaded = gameEngine.image.loadSync?.(path) ?? null;
  if (!loaded) {
    throw new Error(`SlotMachineOverlay requires exported ASM slot UI sheet: ${path}`);
  }
  slotsUiSheetCache = loaded;
  return loaded;
};

const loadSlotsSymbolSheet = (): Surface => {
  const cached = slotsSymbolSheetCache;
  if (cached) {
    return cached;
  }
  const path = resolveSlotsSymbolTilesPath();
  const loaded = gameEngine.image.loadSync?.(path) ?? null;
  if (!loaded) {
    throw new Error(`SlotMachineOverlay requires exported ASM slot symbol sheet: ${path}`);
  }
  slotsSymbolSheetCache = loaded;
  slotIconCache.clear();
  return loaded;
};

const loadSlotsTilemap = (): Uint8Array => {
  const cached = slotsTilemapCache;
  if (cached) {
    return cached;
  }
  const path = resolveSlotsTilemapPath();
  const loaded = fs.readFileSync(path);
  if (loaded.length !== SLOTS_TILEMAP_LENGTH) {
    throw new Error(`Unexpected ASM slots tilemap size: ${loaded.length}`);
  }
  slotsTilemapCache = new Uint8Array(loaded);
  return slotsTilemapCache;
};

const loadSlotsPalettes = (): RGB[][] => {
  const cached = slotsPaletteCache;
  if (cached) {
    return cached;
  }
  const content = fs.readFileSync(resolveSlotsPalettePath(), "utf8");
  const colors = Array.from(content.matchAll(/RGB\s+(\d+),\s*(\d+),\s*(\d+)/g), (match): RGB => [
    gbc5To8(Number(match[1]), "slot red"),
    gbc5To8(Number(match[2]), "slot green"),
    gbc5To8(Number(match[3]), "slot blue"),
  ]);
  if (colors.length !== 16 * 4) {
    throw new Error(`Unexpected ASM slots palette color count: ${colors.length}`);
  }
  slotsPaletteCache = Array.from({ length: 16 }, (_, index) => colors.slice(index * 4, index * 4 + 4));
  return slotsPaletteCache;
};

const paletteIndexFromGray = (gray: number): number => {
  if (gray >= 213) {
    return 0;
  }
  if (gray >= 128) {
    return 1;
  }
  if (gray >= 43) {
    return 2;
  }
  return 3;
};

const paletteForSlotTile = (tileX: number, tileY: number): number => {
  // ASM: engine/gfx/cgb_layouts.asm::_CGB_SlotMachine.
  let palette = 0;
  const fill = (x: number, y: number, width: number, height: number, value: number): void => {
    if (tileX >= x && tileX < x + width && tileY >= y && tileY < y + height) {
      palette = value;
    }
  };
  fill(0, 2, 3, 10, 2);
  fill(17, 2, 3, 10, 2);
  fill(0, 4, 3, 6, 3);
  fill(17, 4, 3, 6, 3);
  fill(0, 6, 3, 2, 4);
  fill(17, 6, 3, 2, 4);
  fill(4, 2, 12, 2, 1);
  fill(3, 2, 1, 10, 1);
  fill(16, 2, 1, 10, 1);
  if (tileY >= 12 && tileY < 18) {
    palette = 7;
  }
  return palette;
};

const applyPaletteToSurface = (
  source: Surface,
  palette: RGB[],
  options: { transparentZero?: boolean } = {},
): Surface => {
  const [width, height] = source.get_size();
  const target = new gameEngine.Surface(width, height);
  const image = source.getImageData();
  const data = image.data;

  for (let i = 0; i < data.length; i += 4) {
    const alpha = data[i + 3];
    if (alpha === 0) {
      continue;
    }
    const paletteIndex = paletteIndexFromGray(data[i]);
    const [r, g, b] = palette[paletteIndex] ?? palette[0];
    data[i] = r;
    data[i + 1] = g;
    data[i + 2] = b;
    data[i + 3] = options.transparentZero && paletteIndex === 0 ? 0 : 255;
  }
  target.getContext()!.putImageData(image, 0, 0);
  return target;
};

const getPaletteSurface = (source: Surface, paletteIndex: number, transparentZero = false): Surface => {
  const key = `${paletteIndex}:${transparentZero ? "t" : "o"}`;
  let cache = slotsPaletteSheetCache.get(source);
  if (!cache) {
    cache = new Map<string, Surface>();
    slotsPaletteSheetCache.set(source, cache);
  }
  const cached = cache.get(key);
  if (cached) {
    return cached;
  }
  const palettes = loadSlotsPalettes();
  const palette = palettes[paletteIndex] ?? palettes[0];
  const surface = applyPaletteToSurface(source, palette, { transparentZero });
  cache.set(key, surface);
  return surface;
};

const blitTile = (screen: Surface, sheet: Surface, tileIndex: number, destX: number, destY: number): void => {
  const columns = Math.floor(sheet.get_width() / TILE_SIZE);
  const rows = Math.floor(sheet.get_height() / TILE_SIZE);
  const totalTiles = columns * rows;
  if (tileIndex < 0 || tileIndex >= totalTiles) {
    throw new Error(`SlotMachineOverlay tile ${tileIndex} is outside exported slots sheet.`);
  }
  screen.blit(sheet, [destX, destY], {
    x: (tileIndex % columns) * TILE_SIZE,
    y: Math.floor(tileIndex / columns) * TILE_SIZE,
    width: TILE_SIZE,
    height: TILE_SIZE,
  });
};

const drawSlotsBackground = (screen: Surface): void => {
  const uiSheet = loadSlotsUiSheet();
  const symbolSheet = loadSlotsSymbolSheet();
  const tilemap = loadSlotsTilemap();
  for (let tileOffset = 0; tileOffset < tilemap.length; tileOffset += 1) {
    const tileId = tilemap[tileOffset];
    const destX = (tileOffset % SLOTS_TILEMAP_WIDTH) * TILE_SIZE;
    const destY = Math.floor(tileOffset / SLOTS_TILEMAP_WIDTH) * TILE_SIZE;
    const paletteIndex = paletteForSlotTile(tileOffset % SLOTS_TILEMAP_WIDTH, Math.floor(tileOffset / SLOTS_TILEMAP_WIDTH));
    if (tileId < SLOTS_VTILES2_OVERLAY_START_TILE) {
      blitTile(screen, getPaletteSurface(uiSheet, paletteIndex), tileId, destX, destY);
    } else {
      blitTile(screen, getPaletteSurface(symbolSheet, paletteIndex), tileId - SLOTS_VTILES2_OVERLAY_START_TILE, destX, destY);
    }
  }
};

const slotIconTileIndices = (baseTileIndex: number): [number, number, number, number] => [
  baseTileIndex,
  baseTileIndex + 1,
  baseTileIndex + 2,
  baseTileIndex + 3,
];

const getSlotIconSurface = (symbol: SlotSymbol): Surface => {
  const cached = slotIconCache.get(symbol);
  if (cached) {
    return cached;
  }
  const sheet = getPaletteSurface(loadSlotsSymbolSheet(), symbol, true);
  const baseTileIndex = symbol * SLOT_ICON_TILE_STRIDE;
  const columns = Math.floor(sheet.get_width() / SLOT_ICON_TILE_SIZE);
  const totalTiles = columns * Math.floor(sheet.get_height() / SLOT_ICON_TILE_SIZE);
  if (baseTileIndex < 0 || baseTileIndex + 3 >= totalTiles) {
    throw new Error(`SlotMachineOverlay symbol ${symbol} is outside exported slot symbol sheet.`);
  }
  const icon = new gameEngine.Surface(SLOT_ICON_SURFACE_SIZE, SLOT_ICON_SURFACE_SIZE);
  const tileLayout = slotIconTileIndices(baseTileIndex);
  const destPositions: Array<[number, number]> = [
    [0, 0],
    [SLOT_ICON_TILE_SIZE, 0],
    [0, SLOT_ICON_TILE_SIZE],
    [SLOT_ICON_TILE_SIZE, SLOT_ICON_TILE_SIZE],
  ];
  for (let i = 0; i < tileLayout.length; i += 1) {
    const tileIndex = tileLayout[i];
    icon.blit(sheet, destPositions[i], {
      x: (tileIndex % columns) * SLOT_ICON_TILE_SIZE,
      y: Math.floor(tileIndex / columns) * SLOT_ICON_TILE_SIZE,
      width: SLOT_ICON_TILE_SIZE,
      height: SLOT_ICON_TILE_SIZE,
    });
  }
  slotIconCache.set(symbol, icon);
  return icon;
};

const initialWindows = (): SlotMachineResult["windows"] => [
  buildVisibleReelWindow(0, REEL_LENGTH - 1),
  buildVisibleReelWindow(1, REEL_LENGTH - 1),
  buildVisibleReelWindow(2, REEL_LENGTH - 1),
];

const wrapReelIndex = (index: number): number => ((index % REEL_LENGTH) + REEL_LENGTH) % REEL_LENGTH;

const buildEngineReelWindow = (reelIndex: number, offset: number): ReelWindow => {
  const reel = REEL_TILEMAPS[reelIndex];
  const start = wrapReelIndex(offset);
  return [
    reel[start],
    reel[wrapReelIndex(start + 1)],
    reel[wrapReelIndex(start + 2)],
  ];
};

const buildVisibleReelWindow = (reelIndex: number, offset: number): ReelWindow => {
  const [bottom, middle, top] = buildEngineReelWindow(reelIndex, offset);
  return [top, middle, bottom];
};

const visibleWindowsFromEngineWindows = (windows: SlotMachineResult["windows"]): ReelWindows => [
  [windows[0][2], windows[0][1], windows[0][0]],
  [windows[1][2], windows[1][1], windows[1][0]],
  [windows[2][2], windows[2][1], windows[2][0]],
];

const buildReelWindowsFromOffsets = (offsets: [number, number, number]): ReelWindows => [
  buildVisibleReelWindow(0, offsets[0]),
  buildVisibleReelWindow(1, offsets[1]),
  buildVisibleReelWindow(2, offsets[2]),
];

const findReelOffsetForWindow = (reelIndex: number, window: ReelWindow): number => {
  const reel = REEL_TILEMAPS[reelIndex];
  for (let offset = 0; offset < REEL_LENGTH; offset += 1) {
    if (
      reel[offset] === window[0] &&
      reel[wrapReelIndex(offset + 1)] === window[1] &&
      reel[wrapReelIndex(offset + 2)] === window[2]
    ) {
      return offset;
    }
  }
  throw new Error(`ASM reel mapping mismatch for reel ${reelIndex}.`);
};

const directionFromEvent = (event: { direction?: unknown; key?: unknown; code?: unknown }): string | null => {
  if (typeof event.direction === "string") {
    return event.direction.toLowerCase();
  }
  const token = String(event.code ?? event.key ?? "").toLowerCase();
  if (token === "arrowleft") return "left";
  if (token === "arrowright") return "right";
  if (token === "arrowup") return "up";
  if (token === "arrowdown") return "down";
  return null;
};

export class SlotMachineOverlay {
  private bet: 1 | 2 | 3;
  private mode: SlotMachineMode;
  private lastResult: SlotMachineResult | null = null;
  private animatedWindows: ReelWindows | null = null;
  private reelOffsets: [number, number, number] = [0, 0, 0];
  private pendingAnimation: PendingSlotAnimation | null = null;
  private closeAfterAnimation = false;
  private message = "PRESS A TO SPIN";
  private played = false;
  private payout = 0;
  private readonly frameAwaiter: () => Promise<void>;
  private readonly stopFrames: readonly [number, number, number];

  constructor(
    private readonly ui: SlotMachineUI,
    private readonly gameState: GameState,
    private readonly audioEngine: AudioEngineLike | null = null,
    options: SlotMachineOverlayOptions = {},
  ) {
    const requestedBet = Math.trunc(Number(options.bet ?? 3));
    this.bet = Math.max(1, Math.min(3, requestedBet)) as 1 | 2 | 3;
    this.mode = options.mode ?? SlotMachineMode.NORMAL;
    this.frameAwaiter = options.frameAwaiter ?? nextFrame;
    this.stopFrames = options.animation?.stopFrames ?? SLOT_REEL_STOP_FRAMES;
  }

  async runAsync(): Promise<SlotMachineOverlayOutcome> {
    this.render();
    while (true) {
      const done = this.stepInput();
      this.render();
      if (done) {
        return this.outcome();
      }
      await this.frameAwaiter();
    }
  }

  private stepInput(): boolean {
    const queue = this.ui.eventQueue;
    if (!queue) {
      return true;
    }
    if (!this.pendingAnimation && this.closeAfterAnimation) {
      return true;
    }
    for (const event of gameEngine.event.get(queue)) {
      if (event.type === gameEngine.QUIT) {
        gameEngine.quit();
        throw new Error("Slot machine interrupted by QUIT event.");
      }
      if (!isKeyDownEvent(event)) {
        continue;
      }
      if (this.pendingAnimation) {
        if (isButtonEvent(event, GameButton.B)) {
          this.closeAfterAnimation = true;
        }
        continue;
      }
      if (isButtonEvent(event, GameButton.B)) {
        playSound(this.audioEngine, "SFX_READ_TEXT_2");
        return true;
      }
      if (isButtonEvent(event, GameButton.A)) {
        this.spin();
        continue;
      }
      const direction = directionFromEvent(event);
      if (direction === "left") {
        this.bet = Math.max(1, this.bet - 1) as 1 | 2 | 3;
        this.message = `BET ${this.bet}`;
        playSound(this.audioEngine, "SFX_READ_TEXT_2");
      } else if (direction === "right") {
        this.bet = Math.min(3, this.bet + 1) as 1 | 2 | 3;
        this.message = `BET ${this.bet}`;
        playSound(this.audioEngine, "SFX_READ_TEXT_2");
      }
    }
    return false;
  }

  private advanceAnimationFrame(): void {
    const animation = this.pendingAnimation;
    if (!animation) {
      return;
    }

    animation.frame += 1;
    const nextOffsets: [number, number, number] = [...this.reelOffsets];
    let allStopped = true;
    for (let reelIndex = 0; reelIndex < animation.stopFrames.length; reelIndex += 1) {
      const stopFrame = animation.stopFrames[reelIndex];
      if (animation.frame >= stopFrame) {
        nextOffsets[reelIndex] = animation.targetOffsets[reelIndex];
        continue;
      }
      allStopped = false;
      nextOffsets[reelIndex] = wrapReelIndex(animation.baseOffsets[reelIndex] + animation.frame + reelIndex * 2);
    }

    this.reelOffsets = nextOffsets;
    this.animatedWindows = buildReelWindowsFromOffsets(nextOffsets);
    if (!allStopped) {
      return;
    }

    this.gameState.sram.coins = animation.nextCoins;
    this.lastResult = animation.result;
    this.animatedWindows = null;
    this.pendingAnimation = null;
    this.played = true;
    this.payout = animation.result.payout;
    this.message = animation.message;
    if (animation.result.matchedSymbol !== null) {
      playSound(
        this.audioEngine,
        animation.result.matchedSymbol === SlotSymbol.SEVEN
          ? "SFX_2ND_PLACE"
          : animation.result.matchedSymbol === SlotSymbol.POKEBALL
            ? "SFX_3RD_PLACE"
            : "SFX_PRESENT",
      );
    }
  }

  private spin(): void {
    if (this.pendingAnimation) {
      return;
    }
    const coins = Number(this.gameState.sram.coins ?? 0);
    if (coins < this.bet) {
      this.message = "NEED MORE COINS";
      playSound(this.audioEngine, "SFX_WRONG");
      return;
    }

    playSound(this.audioEngine, "SFX_SLOT_MACHINE_START");
    const machine = new SlotMachine(new HardwareRNG(this.gameState));
    const result = machine.spin({ bet: this.bet, mode: this.mode });
    const nextCoins = clampCoins(coins - this.bet + result.payout);
    this.pendingAnimation = {
      frame: 0,
      baseOffsets: [...this.reelOffsets],
      targetOffsets: [
        findReelOffsetForWindow(0, result.windows[0]),
        findReelOffsetForWindow(1, result.windows[1]),
        findReelOffsetForWindow(2, result.windows[2]),
      ],
      result,
      nextCoins,
      message: result.payout > 0 ? `WIN ${result.payout}` : "DARN",
      stopFrames: this.stopFrames,
    };
    this.message = "START!";
  }

  private outcome(): SlotMachineOverlayOutcome {
    const result = this.lastResult;
    return {
      played: this.played,
      bet: this.bet,
      payout: this.payout,
      matched_symbol:
        result?.matchedSymbol !== null && result?.matchedSymbol !== undefined
          ? (SlotSymbol[result.matchedSymbol] as keyof typeof SlotSymbol)
          : null,
      winning_lines: result?.winningLines ?? [],
      coins: Number(this.gameState.sram.coins ?? 0),
    };
  }

  private render(): void {
    this.advanceAnimationFrame();
    const windows =
      this.animatedWindows ??
      (this.lastResult ? visibleWindowsFromEngineWindows(this.lastResult.windows) : initialWindows());
    const rows = [0, 1, 2].map((rowIndex) =>
      windows.map((window) => SYMBOL_LABELS[window[rowIndex]]).join(" | "),
    );
    this.renderScreen(windows);
    renderTextSnapshot(this.ui, {
      viewportTitle: "Slot Machine",
      infoTitle: "Legend",
      viewportLines: [
        "SLOT MACHINE",
        `COINS ${Number(this.gameState.sram.coins ?? 0)}`,
        `BET ${this.bet}`,
        "",
        ...rows,
        "",
        this.message,
      ],
      infoLines: [
        "STATE: slot_machine",
        "Left/Right=Bet A=Spin B=Quit",
      ],
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
    });
    this.ui.update?.();
  }

  private renderScreen(windows: SlotMachineResult["windows"]): void {
    const screen = this.ui.screen as Surface | undefined;
    if (!screen) {
      return;
    }

    screen.fill([255, 255, 255, 255]);
    drawSlotsBackground(screen);
    renderPixelText(
      this.ui,
      String(Number(this.gameState.sram.coins ?? 0)).padStart(4, "0"),
      SLOT_COIN_COUNT_TILE_X * TILE_SIZE,
      SLOT_COUNTER_TILE_Y * TILE_SIZE,
    );
    renderPixelText(
      this.ui,
      String(this.payout).padStart(4, "0"),
      SLOT_PAYOUT_TILE_X * TILE_SIZE,
      SLOT_COUNTER_TILE_Y * TILE_SIZE,
    );
    renderPixelText(
      this.ui,
      this.message,
      SLOT_PROMPT_TILE_X * TILE_SIZE,
      SLOT_PROMPT_TILE_Y * TILE_SIZE,
    );
    for (let reel = 0; reel < 3; reel += 1) {
      for (let row = 0; row < 3; row += 1) {
        const symbol = windows[reel][row];
        screen.blit(getSlotIconSurface(symbol), [
          SLOT_REEL_X_TILES[reel] * TILE_SIZE,
          SLOT_REEL_Y_TILES[row] * TILE_SIZE,
        ]);
      }
    }
  }
}
