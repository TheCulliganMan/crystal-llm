
import { getAssetPath } from '@pokecrystal/core/core/paths';
import { getMapEnvironment } from '@pokecrystal/core/engine/world/maps';
import { gameEngine, Surface, Rect } from '@pokecrystal/core/ui/game-engine';
import fs from 'fs';
import { Random } from '@pokecrystal/core/core/random';
import { decode2bppTiles } from '@pokecrystal/core/ui/2bpp';
import { gbc5To8 } from '@pokecrystal/core/core/gbc-colors';

type RGBTuple = [number, number, number];
type Palette = RGBTuple[];

// Screen/tile constants pulled directly from the disassembly.
const SCREEN_WIDTH_TILES = 20;
const SCREEN_HEIGHT_TILES = 18;
const WIDE_PADDING_TILES = 4;

// Tile ids used by the ASM routine.
const BATTLETRANSITION_SQUARE = 0xfe;
const BATTLETRANSITION_BLACK = 0xff;

// Flash palette steps (crumb encodings) straight from the table in the ASM.
const FLASH_CRUMBS: [number, number, number, number][] = [
  [3, 3, 2, 1],
  [3, 3, 3, 2],
  [3, 3, 3, 3],
  [3, 3, 3, 2],
  [3, 3, 2, 1],
  [3, 2, 1, 0],
  [2, 1, 0, 0],
  [1, 0, 0, 0],
  [0, 0, 0, 0],
  [1, 0, 0, 0],
  [2, 1, 0, 0],
  [3, 2, 1, 0],
  [0, 0, 0, 1],
];

// The 16×16 poke ball mask the ASM iterates bit-by-bit.
const POKEBALL_PATTERN: string[] = [
  '......XXXX......',
  '....XXXXXXXX....',
  '..XXXX....XXXX..',
  '..XX........XX..',
  '.XX..........XX.',
  '.XX...XXXX...XX.',
  'XX...XX..XX...XX',
  'XXXXXX....XXXXXX',
  'XXXXXX....XXXXXX',
  'XX...XX..XX...XX',
  '.XX...XXXX...XX.',
  '.XX..........XX.',
  '..XX........XX..',
  '..XXXX....XXXX..',
  '....XXXXXXXX....',
  '......XXXX......',
];

// Quadrant flags used by the spin wedge routine.
const RIGHT_QUADRANT_FLAG = 1 << 0;
const LOWER_QUADRANT_FLAG = 1 << 1;

function packPalette(c0: number, c1: number, c2: number, c3: number): number {
  return ((c0 & 0x03) << 6) | ((c1 & 0x03) << 4) | ((c2 & 0x03) << 2) | (c3 & 0x03);
}

const FLASH_TABLE_BGP: number[] = FLASH_CRUMBS.map(crumbs => packPalette(...crumbs));

const DEFAULT_PALETTE: RGBTuple[] = [[248, 248, 248]];
const DEFAULT_TILE_PALETTE: Palette = [
  [248, 248, 248],
  [168, 168, 168],
  [80, 80, 80],
  [0, 0, 0],
];
const PALETTE_CACHE = new Map<string, RGBTuple[]>();

function parsePalette(path: string): RGBTuple[] {
  const cached = PALETTE_CACHE.get(path);
  if (cached) {
    return cached;
  }
  const colors: RGBTuple[] = [];
  if (!fs.existsSync(path)) {
    PALETTE_CACHE.set(path, colors);
    return colors;
  }
  const raw = fs.readFileSync(path, 'utf-8');
  for (const line of raw.split('\n')) {
    const trimmedLine = line.trim();
    if (!trimmedLine || trimmedLine.startsWith(';')) {
      continue;
    }
    if (!trimmedLine.toUpperCase().includes('RGB')) {
      continue;
    }
    try {
      const segment = trimmedLine.toUpperCase().split('RGB')[1];
      const parts = segment.split(',').map(p => p.trim());
      if (parts.length < 3) {
        continue;
      }
      const values = parts
        .slice(0, 3)
        .map(p => Math.max(0, Math.min(31, parseInt(p, 10))));
      const converted: RGBTuple = [
        gbc5To8(values[0], `${path} r`),
        gbc5To8(values[1], `${path} g`),
        gbc5To8(values[2], `${path} b`),
      ];
      colors.push(converted);
    } catch (e) {
      continue;
    }
  }
  PALETTE_CACHE.set(path, colors);
  return colors;
}

export enum BattleTransitionState {
  DETERMINE_ANIMATION = 0x00,

  // CAVE
  CAVE_LOAD_GFX = 0x01,
  CAVE_SETUP_BGMAP = 0x02,
  CAVE_FLASH_1 = 0x03,
  CAVE_FLASH_2 = 0x04,
  CAVE_FLASH_3 = 0x05,
  CAVE_NEXT_SCENE = 0x06,
  CAVE_SETUP_WAVY = 0x07,
  CAVE_WAVE = 0x08,

  // CAVE STRONGER
  CAVE_STRONGER_LOAD_GFX = 0x09,
  CAVE_STRONGER_SETUP_BGMAP = 0x0a,
  CAVE_STRONGER_FLASH_1 = 0x0b,
  CAVE_STRONGER_FLASH_2 = 0x0c,
  CAVE_STRONGER_FLASH_3 = 0x0d,
  CAVE_STRONGER_NEXT_SCENE = 0x0e,
  CAVE_STRONGER_ZOOM = 0x0f,

  // NO CAVE
  NO_CAVE_LOAD_GFX = 0x10,
  NO_CAVE_SETUP_BGMAP = 0x11,
  NO_CAVE_FLASH_1 = 0x12,
  NO_CAVE_FLASH_2 = 0x13,
  NO_CAVE_FLASH_3 = 0x14,
  NO_CAVE_NEXT_SCENE = 0x15,
  NO_CAVE_SETUP_SPIN = 0x16,
  NO_CAVE_SPIN = 0x17,

  // NO CAVE STRONGER
  NO_CAVE_STRONGER_LOAD_GFX = 0x18,
  NO_CAVE_STRONGER_SETUP_BGMAP = 0x19,
  NO_CAVE_STRONGER_FLASH_1 = 0x1a,
  NO_CAVE_STRONGER_FLASH_2 = 0x1b,
  NO_CAVE_STRONGER_FLASH_3 = 0x1c,
  NO_CAVE_STRONGER_NEXT_SCENE = 0x1d,
  NO_CAVE_STRONGER_SETUP_SCATTER = 0x1e,
  NO_CAVE_STRONGER_SCATTER = 0x1f,

  FINISH = 0x20,
}

const TRANS_STRONGER_FLAG = 1;
const TRANS_NO_CAVE_FLAG = 1 << 1;

interface SpinQuadrantEntry {
  quadrant: number;
  wedge: number[];
  x: number;
  y: number;
}

const WEDGE_1: number[] = [2, 3, 5, 4, 9, -1];
const WEDGE_2: number[] = [1, 1, 2, 2, 4, 2, 4, 2, 3, -1];
const WEDGE_3: number[] = [2, 1, 3, 1, 4, 1, 4, 1, 4, 1, 3, 1, 2, 1, 1, 1, 1, -1];
const WEDGE_4: number[] = [4, 1, 4, 0, 3, 1, 3, 0, 2, 1, 2, 0, 1, -1];
const WEDGE_5: number[] = [4, 0, 3, 0, 3, 0, 2, 0, 2, 0, 1, 0, 1, 0, 1, -1];

const SPIN_QUADRANTS: SpinQuadrantEntry[] = [
    { quadrant: 0, wedge: WEDGE_1, x: 1, y: 6 },
    { quadrant: 0, wedge: WEDGE_2, x: 0, y: 3 },
    { quadrant: 0, wedge: WEDGE_3, x: 1, y: 0 },
    { quadrant: 0, wedge: WEDGE_4, x: 5, y: 0 },
    { quadrant: 0, wedge: WEDGE_5, x: 9, y: 0 },
    { quadrant: 1, wedge: WEDGE_5, x: 10, y: 0 },
    { quadrant: 1, wedge: WEDGE_4, x: 14, y: 0 },
    { quadrant: 1, wedge: WEDGE_3, x: 18, y: 0 },
    { quadrant: 1, wedge: WEDGE_2, x: 19, y: 3 },
    { quadrant: 1, wedge: WEDGE_1, x: 18, y: 6 },
    { quadrant: 3, wedge: WEDGE_1, x: 18, y: 11 },
    { quadrant: 3, wedge: WEDGE_2, x: 19, y: 14 },
    { quadrant: 3, wedge: WEDGE_3, x: 18, y: 17 },
    { quadrant: 3, wedge: WEDGE_4, x: 14, y: 17 },
    { quadrant: 3, wedge: WEDGE_5, x: 10, y: 17 },
    { quadrant: 2, wedge: WEDGE_5, x: 9, y: 17 },
    { quadrant: 2, wedge: WEDGE_4, x: 5, y: 17 },
    { quadrant: 2, wedge: WEDGE_3, x: 1, y: 17 },
    { quadrant: 2, wedge: WEDGE_2, x: 0, y: 14 },
    { quadrant: 2, wedge: WEDGE_1, x: 1, y: 11 },
];

const ZOOM_BOXES: [number, number, number, number][] = [
  [4, 2, 8, 8],
  [6, 4, 7, 7],
  [8, 6, 6, 6],
  [10, 8, 5, 5],
  [12, 10, 4, 4],
  [14, 12, 3, 3],
  [16, 14, 2, 2],
  [18, 16, 1, 1],
  [20, 18, 0, 0],
];

function encodePokeballBytes(pattern: string[]): number[] {
  function bitsToByte(bits: string): number {
    let value = 0;
    for (const char of bits) {
      value = (value << 1) | (char.toUpperCase() === 'X' ? 1 : 0);
    }
    return value;
  }

  const bytesOut: number[] = [];
  for (const line of pattern) {
    if (line.length !== 16) {
      throw new Error(`Invalid poke ball mask row '${line}'. Expected length 16.`);
    }
    bytesOut.push(bitsToByte(line.substring(0, 8)));
    bytesOut.push(bitsToByte(line.substring(8)));
  }
  return bytesOut;
}

const POKEBALL_BYTES = encodePokeballBytes(POKEBALL_PATTERN);

class BattleTransitionError extends Error {
  public readonly state: BattleTransitionState;

  constructor(message: string, state: BattleTransitionState) {
    super(message);
    this.name = "BattleTransitionError";
    this.state = state;
  }
}

class GraphicsLoadError extends BattleTransitionError {
  public readonly resource: string;

  constructor(message: string, state: BattleTransitionState, resource: string) {
    super(message, state);
    this.name = "GraphicsLoadError";
    this.resource = resource;
  }
}

type BattleTransitionGraphics = {
  palette: Palette;
  ringPalette: Palette;
  defaultColor: RGBTuple;
  squareTile: Surface;
  blackTile: Surface;
  tileWidth: number;
};

export class BattleTransitionManager {
  private screen: Surface | null;
  private isTrainer: boolean;
  private playerLevel: number;
  private enemyLevel: number;
  private mapName: string;

  private jumptableIndex: BattleTransitionState;
  private stateCounter: number;
  private sineWaveOffset: number;
  private sineAmplitude: number;
  private pendingFrames: number;
  private complete: boolean;

  private palette: RGBTuple[];
  private ringPalette: RGBTuple[];
  private defaultColor: RGBTuple;
  private tileWidth: number;
  private tileMap: number[][];
  private tileSurfaces: Map<number, Surface>;
  private squareTile: Surface | null;
  private blackTile: Surface | null;
  private backgroundSurface: Surface | null;
  private wideBackground: Surface | null;
  private screenRect: Rect;
  private graphicsReady: boolean;
  private tilemapDirty: boolean;

  private spinIndex: number;
  private zoomIndex: number;
  private scatterCooldownFrames: number;
  private currentPaletteValue: number;
  private rng: Random;

  constructor(
    screen: Surface | null,
    options: {
      isTrainerBattle: boolean;
      playerLevel: number;
      enemyLevel: number;
      mapName?: string;
    }
  ) {
    this.screen = screen;
    this.isTrainer = options.isTrainerBattle;
    this.playerLevel = Math.max(1, options.playerLevel);
    this.enemyLevel = Math.max(1, options.enemyLevel);
    this.mapName = options.mapName || '';

    this.jumptableIndex = BattleTransitionState.DETERMINE_ANIMATION;
    this.stateCounter = 0;
    this.sineWaveOffset = 0;
    this.sineAmplitude = 0;
    this.pendingFrames = 0;
    this.complete = screen === null;

    this.palette = [];
    this.ringPalette = [];
    this.defaultColor = [0, 0, 0];
    this.tileWidth = 8;
    this.tileMap = Array(SCREEN_HEIGHT_TILES)
      .fill(0)
      .map(() => Array(SCREEN_WIDTH_TILES).fill(BATTLETRANSITION_SQUARE));
    this.tileSurfaces = new Map();
    this.squareTile = null;
    this.blackTile = null;
    this.backgroundSurface = null;
    this.wideBackground = null;
    this.screenRect = new gameEngine.Rect(0, 0, 0, 0);
    this.graphicsReady = false;
    this.tilemapDirty = true;

    this.spinIndex = 0;
    this.zoomIndex = 0;
    this.scatterCooldownFrames = 0;
    this.currentPaletteValue = FLASH_TABLE_BGP[0];
    this.rng = new Random(0);

    if (this.screen !== null) {
      this.prepareGraphics();
      this.graphicsReady = true;
    }
  }

  public get currentState(): BattleTransitionState {
    return this.jumptableIndex;
  }

  public isComplete(): boolean {
    return this.complete;
  }

  public consumeCompletion(): boolean {
    return false;
  }

  public advance(): void {
    if (this.complete) {
      return;
    }

    if (this.pendingFrames > 0) {
      this.pendingFrames -= 1;
      return;
    }

    const state = this.jumptableIndex;
    const extraDelayFrames = this.stepState(state);
    this.pendingFrames = Math.max(0, extraDelayFrames);
  }

  private stepState(state: BattleTransitionState): number {
    switch (state) {
      case BattleTransitionState.DETERMINE_ANIMATION:
        return this.determineAnimation();

      case BattleTransitionState.CAVE_LOAD_GFX:
      case BattleTransitionState.CAVE_STRONGER_LOAD_GFX:
      case BattleTransitionState.NO_CAVE_LOAD_GFX:
      case BattleTransitionState.NO_CAVE_STRONGER_LOAD_GFX:
        return this.loadPokeballGraphics();

      case BattleTransitionState.CAVE_SETUP_BGMAP:
      case BattleTransitionState.CAVE_STRONGER_SETUP_BGMAP:
      case BattleTransitionState.NO_CAVE_SETUP_BGMAP:
      case BattleTransitionState.NO_CAVE_STRONGER_SETUP_BGMAP:
        return this.setupBgmap();

      case BattleTransitionState.CAVE_FLASH_1:
      case BattleTransitionState.CAVE_FLASH_2:
      case BattleTransitionState.CAVE_FLASH_3:
      case BattleTransitionState.CAVE_STRONGER_FLASH_1:
      case BattleTransitionState.CAVE_STRONGER_FLASH_2:
      case BattleTransitionState.CAVE_STRONGER_FLASH_3:
      case BattleTransitionState.NO_CAVE_FLASH_1:
      case BattleTransitionState.NO_CAVE_FLASH_2:
      case BattleTransitionState.NO_CAVE_FLASH_3:
      case BattleTransitionState.NO_CAVE_STRONGER_FLASH_1:
      case BattleTransitionState.NO_CAVE_STRONGER_FLASH_2:
      case BattleTransitionState.NO_CAVE_STRONGER_FLASH_3:
        return this.flashScreen();

      case BattleTransitionState.CAVE_NEXT_SCENE:
      case BattleTransitionState.CAVE_STRONGER_NEXT_SCENE:
      case BattleTransitionState.NO_CAVE_NEXT_SCENE:
      case BattleTransitionState.NO_CAVE_STRONGER_NEXT_SCENE:
        return this.nextScene();

      case BattleTransitionState.CAVE_SETUP_WAVY:
        return this.setupWavyOutro();

      case BattleTransitionState.CAVE_WAVE:
        return this.sineWave();

      case BattleTransitionState.CAVE_STRONGER_ZOOM:
        return this.zoomToBlack();

      case BattleTransitionState.NO_CAVE_SETUP_SPIN:
        return this.setupSpinOutro();

      case BattleTransitionState.NO_CAVE_SPIN:
        return this.spinToBlack();

      case BattleTransitionState.NO_CAVE_STRONGER_SETUP_SCATTER:
        return this.setupScatterOutro();

      case BattleTransitionState.NO_CAVE_STRONGER_SCATTER:
        return this.speckleToBlack();

      case BattleTransitionState.FINISH:
        return this.finishState();

      default:
        throw new BattleTransitionError(`Unhandled battle transition state ${state}`, state);
    }
  }

  public draw(): void {
    if (this.screen === null || !this.graphicsReady || this.complete) {
      return;
    }

    this.requireScreen();
    const state = this.jumptableIndex;

    if (state !== BattleTransitionState.DETERMINE_ANIMATION) {
      this.drawBaseBackground();
    }

    if (this.isFlashState(state)) {
      this.drawFlashOverlay();
    } else if (state === BattleTransitionState.CAVE_WAVE) {
      this.drawSineWaveEffect();
    }
  }

  // StartTrainerBattle_DetermineWhichAnimation
  private determineAnimation(): number {
    let flags = 0;
    if (this.playerLevel + 3 < this.enemyLevel) {
      flags |= TRANS_STRONGER_FLAG;
    }

    if (!this.isCaveEnvironment()) {
      flags |= TRANS_NO_CAVE_FLAG;
    }

    const startingPoints: { [key: number]: BattleTransitionState } = {
      0: BattleTransitionState.CAVE_LOAD_GFX,
      [TRANS_STRONGER_FLAG]: BattleTransitionState.CAVE_STRONGER_LOAD_GFX,
      [TRANS_NO_CAVE_FLAG]: BattleTransitionState.NO_CAVE_LOAD_GFX,
      [TRANS_STRONGER_FLAG | TRANS_NO_CAVE_FLAG]: BattleTransitionState.NO_CAVE_STRONGER_LOAD_GFX,
    };
    this.jumptableIndex = startingPoints[flags] || BattleTransitionState.CAVE_LOAD_GFX;
    return 0;
  }

  private isCaveEnvironment(): boolean {
    const environment = getMapEnvironment(this.mapName);
    if (!environment) {
      return false;
    }
    return ["CAVE", "ENVIRONMENT_5", "DUNGEON"].includes(environment.toUpperCase());
  }

  private loadPokeballGraphics(): number {
    if (!this.isTrainer) {
      return this.nextScene();
    }

    if (this.squareTile === null || this.blackTile === null) {
      throw new Error('Poké Ball tiles are not ready');
    }

    this.tileMap = Array(SCREEN_HEIGHT_TILES)
      .fill(0)
      .map(() => Array(SCREEN_WIDTH_TILES).fill(BATTLETRANSITION_BLACK));
    this.paintPokeballMask();
    this.tilemapDirty = true;
    this.nextScene();
    return 1;
  }

  private nextScene(): number {
    const nextValue = this.jumptableIndex + 1;
    if (BattleTransitionState[nextValue] !== undefined) {
      this.jumptableIndex = nextValue;
    } else {
      this.jumptableIndex = BattleTransitionState.FINISH;
    }
    return 0;
  }

  private setupBgmap(): number {
    this.nextScene();
    this.stateCounter = 0;
    return 0;
  }

  private flashScreen(): number {
    const finished = this.doFlashAnimation();
    if (finished) {
      this.nextScene();
    }
    return 0;
  }

  // ASM mapping: StartTrainerBattle_Flash.DoFlashAnimation
  // - uses the pre-increment counter value for the `srl a` lookup
  // - then increments wBattleTransitionCounter.
  private doFlashAnimation(): boolean {
    const counterBeforeIncrement = this.stateCounter;
    this.stateCounter = (this.stateCounter + 1) & 0xff;
    const index = counterBeforeIncrement >> 1;
    if (index >= FLASH_TABLE_BGP.length) {
      this.stateCounter = 0;
      return true;
    }

    this.currentPaletteValue = FLASH_TABLE_BGP[index];
    if (this.currentPaletteValue === 0x01) {
      this.stateCounter = 0;
      return true;
    }
    return false;
  }

  private setupWavyOutro(): number {
    this.nextScene();
    this.stateCounter = 0;
    this.sineWaveOffset = 0;
    this.sineAmplitude = 0;
    return 0;
  }

  // ASM mapping: StartTrainerBattle_SineWave.DoSineWave
  // offset is incremented first and that incremented offset contributes
  // to the next counter value.
  private sineWave(): number {
    if (this.stateCounter >= 0x60) {
      this.jumptableIndex = BattleTransitionState.FINISH;
      return 0;
    }

    const previousCounter = this.stateCounter;
    this.sineAmplitude = previousCounter;

    this.sineWaveOffset = (this.sineWaveOffset + 1) & 0xff;
    this.stateCounter = (previousCounter + this.sineWaveOffset) & 0xff;
    return 0;
  }

  private zoomToBlack(): number {
    if (this.zoomIndex < ZOOM_BOXES.length) {
      const [width, height, startY, startX] = ZOOM_BOXES[this.zoomIndex];
      this.fillBox(startX, startY, width, height);
      this.zoomIndex += 1;
      if (this.zoomIndex >= ZOOM_BOXES.length) {
        this.jumptableIndex = BattleTransitionState.FINISH;
        return 1;
      }
      return 0;
    }

    this.jumptableIndex = BattleTransitionState.FINISH;
    return 1;
  }

  private setupSpinOutro(): number {
    this.nextScene();
    this.stateCounter = 0;
    this.spinIndex = 0;
    return 0;
  }

  private spinToBlack(): number {
    if (this.spinIndex >= SPIN_QUADRANTS.length) {
      this.jumptableIndex = BattleTransitionState.FINISH;
      return 3;
    }

    const entry = SPIN_QUADRANTS[this.spinIndex];
    this.stateCounter = this.spinIndex;
    this.applySpinWedge(entry);
    this.spinIndex += 1;
    return 2;
  }

  private setupScatterOutro(): number {
    this.nextScene();
    this.stateCounter = 0x10;
    this.scatterCooldownFrames = 0;
    return 0;
  }

  private speckleToBlack(): number {
    if (this.stateCounter <= 0) {
      if (this.scatterCooldownFrames === 0) {
        this.scatterCooldownFrames = 3;
        return this.scatterCooldownFrames;
      }
      this.jumptableIndex = BattleTransitionState.FINISH;
      return 0;
    }

    this.stateCounter -= 1;
    for (let i = 0; i < 12; i += 1) {
      this.blackOutRandomTile();
    }
    return 0;
  }

  private finishState(): number {
    this.complete = true;
    return 0;
  }

  // --- Drawing helpers ---
  private isFlashState(state: BattleTransitionState): boolean {
    return (
      state === BattleTransitionState.CAVE_FLASH_1 ||
      state === BattleTransitionState.CAVE_FLASH_2 ||
      state === BattleTransitionState.CAVE_FLASH_3 ||
      state === BattleTransitionState.CAVE_STRONGER_FLASH_1 ||
      state === BattleTransitionState.CAVE_STRONGER_FLASH_2 ||
      state === BattleTransitionState.CAVE_STRONGER_FLASH_3 ||
      state === BattleTransitionState.NO_CAVE_FLASH_1 ||
      state === BattleTransitionState.NO_CAVE_FLASH_2 ||
      state === BattleTransitionState.NO_CAVE_FLASH_3 ||
      state === BattleTransitionState.NO_CAVE_STRONGER_FLASH_1 ||
      state === BattleTransitionState.NO_CAVE_STRONGER_FLASH_2 ||
      state === BattleTransitionState.NO_CAVE_STRONGER_FLASH_3
    );
  }

  private drawBaseBackground(): void {
    this.ensureBackgroundSurfaces();
    const screen = this.requireScreen();
    if (this.backgroundSurface !== null) {
      screen.blit(this.backgroundSurface, [0, 0]);
    } else {
      const color = this.defaultColor;
      screen.fill([color[0], color[1], color[2], 255]);
    }
  }

  private drawFlashOverlay(): void {
    const bgpValue = this.currentPaletteValue;
    const shades = [
      (bgpValue >> 0) & 0x03,
      (bgpValue >> 2) & 0x03,
      (bgpValue >> 4) & 0x03,
      (bgpValue >> 6) & 0x03,
    ];
    const averageShade = shades.reduce((sum, value) => sum + value, 0) / shades.length;
    const alpha = Math.trunc(((3.0 - averageShade) / 3.0) * 255);
    if (alpha <= 0) {
      return;
    }
    const color = this.palette[0] ?? [248, 248, 248];
    this.overlayColor(color, alpha);
  }

  private drawSineWaveEffect(): void {
    this.ensureBackgroundSurfaces();
    if (this.wideBackground === null) {
      return;
    }

    const screen = this.requireScreen();
    const screenW = this.screenRect.width;
    const screenH = this.screenRect.height;
    const paddingPx = WIDE_PADDING_TILES * this.tileWidth;
    const amplitude = Math.max(1, Math.min(12, Math.trunc(this.sineAmplitude / 2) + 2));
    const phaseBase = ((this.sineWaveOffset & 0x3f) * Math.PI) / 32.0;

    for (let y = 0; y < screenH; y += 2) {
      const phase = phaseBase + (y * Math.PI) / 128.0;
      const shift = Math.trunc(Math.sin(phase) * amplitude);
      const srcX = paddingPx + shift;
      const srcRect = new gameEngine.Rect(srcX, y, screenW, 2);
      screen.blit(this.wideBackground, [0, y], srcRect);
    }
  }

  private overlayColor(color: RGBTuple, alpha: number): void {
    if (this.screen === null) {
      return;
    }
    const overlay = new gameEngine.Surface(this.screenRect.width, this.screenRect.height);
    overlay.fill([color[0], color[1], color[2], Math.max(0, Math.min(255, alpha))]);
    const screen = this.requireScreen();
    screen.blit(overlay, [0, 0]);
  }

  private prepareGraphics(): void {
    if (this.screen === null) {
      return;
    }

    const graphics = this.loadGraphicsOrThrow();
    this.applyGraphics(graphics);
  }

  private loadGraphicsOrThrow(): BattleTransitionGraphics {
    const tilePath = getAssetPath("gfx", "overworld", "trainer_battle_pokeball_tiles.2bpp");
    const palettePath = getAssetPath("gfx", "overworld", "trainer_battle.pal");
    const darkPalettePath = getAssetPath("gfx", "overworld", "trainer_battle_dark.pal");

    let tileData: Buffer;
    try {
      tileData = fs.readFileSync(tilePath);
    } catch (e: any) {
      throw new GraphicsLoadError(
        `Unable to read tile data: ${e?.message ?? String(e)}`,
        this.jumptableIndex,
        "trainer_battle_pokeball_tiles.2bpp"
      );
    }

    // Two 8x8 2bpp tiles = 32 bytes.
    if (tileData.length !== 32) {
      throw new GraphicsLoadError(
        `Unexpected pokeball tile payload length ${tileData.length}.`,
        this.jumptableIndex,
        "trainer_battle_pokeball_tiles.2bpp"
      );
    }

    const parsedPalette = parsePalette(palettePath);
    if (!this.validatePalette(parsedPalette)) {
      throw new GraphicsLoadError(
        "Invalid palette format",
        this.jumptableIndex,
        "trainer_battle.pal"
      );
    }

    const parsedRing = parsePalette(darkPalettePath);
    const ringPalette = this.validatePalette(parsedRing) ? parsedRing : parsedPalette;

    const tiles = decode2bppTiles(tileData, parsedPalette.length ? parsedPalette : DEFAULT_TILE_PALETTE);
    if (!this.validateTiles(tiles)) {
      throw new GraphicsLoadError(
        "Pokeball tiles did not decode correctly.",
        this.jumptableIndex,
        "trainer_battle_pokeball_tiles.2bpp"
      );
    }

    return {
      palette: parsedPalette,
      ringPalette,
      defaultColor: parsedPalette[0] ?? DEFAULT_PALETTE[0],
      squareTile: tiles[0],
      blackTile: tiles[1],
      tileWidth: tiles[0].get_width(),
    };
  }

  private applyGraphics(graphics: BattleTransitionGraphics): void {
    const screen = this.requireScreen();
    this.screenRect = new gameEngine.Rect(0, 0, screen.get_width(), screen.get_height());

    this.squareTile = graphics.squareTile;
    this.blackTile = graphics.blackTile;
    this.tileSurfaces = new Map([
      [BATTLETRANSITION_SQUARE, this.squareTile],
      [BATTLETRANSITION_BLACK, this.blackTile],
    ]);

    this.tileWidth = graphics.tileWidth;
    this.palette = graphics.palette;
    this.ringPalette = graphics.ringPalette;
    this.defaultColor = graphics.defaultColor;
    this.tilemapDirty = true;
    this.ensureBackgroundSurfaces();
  }

  private validatePalette(palette: unknown): palette is Palette {
    if (!Array.isArray(palette) || palette.length !== 4) {
      return false;
    }
    return palette.every(entry => {
      if (!Array.isArray(entry) || entry.length !== 3) {
        return false;
      }
      return entry.every(channel => Number.isFinite(channel) && channel >= 0 && channel <= 255);
    });
  }

  private validateTiles(tiles: unknown): tiles is Surface[] {
    if (!Array.isArray(tiles) || tiles.length < 2) {
      return false;
    }
    const first = tiles[0] as any;
    const second = tiles[1] as any;
    return (
      first &&
      typeof first.get_width === "function" &&
      typeof first.get_height === "function" &&
      second &&
      typeof second.get_width === "function" &&
      typeof second.get_height === "function" &&
      first.get_width() === 8 &&
      first.get_height() === 8 &&
      second.get_width() === 8 &&
      second.get_height() === 8
    );
  }

  private markTilemapDirty(): void {
    this.tilemapDirty = true;
  }

  private ensureBackgroundSurfaces(): void {
    if (!this.tilemapDirty) {
      return;
    }

    this.backgroundSurface = this.renderTilemapSurface(0);
    this.wideBackground = this.renderTilemapSurface(WIDE_PADDING_TILES);
    this.tilemapDirty = false;
  }

  private renderTilemapSurface(paddingTiles: number): Surface {
    const widthTiles = SCREEN_WIDTH_TILES + paddingTiles * 2;
    const width = widthTiles * this.tileWidth;
    const height = SCREEN_HEIGHT_TILES * this.tileWidth;
    const surface = new gameEngine.Surface(width, height);
    const color = this.defaultColor;
    surface.fill([color[0], color[1], color[2], 255]);
    const xOffset = paddingTiles * this.tileWidth;

    for (let y = 0; y < this.tileMap.length; y += 1) {
      const row = this.tileMap[y];
      for (let x = 0; x < row.length; x += 1) {
        const tileId = row[x];
        const tileSurface = this.tileSurfaces.get(tileId) ?? this.squareTile;
        if (!tileSurface) {
          continue;
        }
        surface.blit(tileSurface, [x * this.tileWidth + xOffset, y * this.tileWidth]);
      }
    }
    return surface;
  }

  private paintPokeballMask(): void {
    if (this.squareTile === null) {
      return;
    }
    if (POKEBALL_BYTES.length !== POKEBALL_PATTERN.length * 2) {
      throw new Error("Unexpected poke ball mask length.");
    }

    const hlX = 2;
    let hlY = 1;
    let dataIndex = 0;

    for (let row = 0; row < POKEBALL_PATTERN.length; row += 1) {
      let x = hlX;
      const y = hlY;
      for (let segment = 0; segment < 2; segment += 1) {
        const byteValue = POKEBALL_BYTES[dataIndex];
        dataIndex += 1;
        let working = byteValue & 0xff;
        // The ASM uses SLA and checks carry for each of 8 bits. Always iterate 8 times,
        // even when the remaining bits are 0, to keep X alignment correct.
        for (let bit = 0; bit < 8; bit += 1) {
          const carry = (working & 0x80) !== 0;
          working = (working << 1) & 0xff;
          if (carry) {
            this.setTile(x, y, BATTLETRANSITION_SQUARE);
          }
          x += 1;
        }
        x = hlX + Math.trunc((SCREEN_WIDTH_TILES - 4) / 2);
      }
      hlY += 1;
    }
  }

  private applySpinWedge(entry: SpinQuadrantEntry): void {
    const drawDirection = entry.quadrant & RIGHT_QUADRANT_FLAG ? 1 : -1;
    const offsetDirection = -drawDirection;
    const verticalDelta = entry.quadrant & LOWER_QUADRANT_FLAG ? -1 : 1;

    let x = entry.x;
    let y = entry.y;
    const wedge = entry.wedge;
    let index = 0;

    while (index < wedge.length) {
      const runLength = wedge[index];
      index += 1;
      if (runLength < 0) {
        break;
      }

      const baseX = x;
      const baseY = y;
      for (let i = 0; i < runLength; i += 1) {
        this.setTile(x, y, BATTLETRANSITION_BLACK);
        x += drawDirection;
      }
      x = baseX;
      y = baseY + verticalDelta;

      if (index >= wedge.length) {
        break;
      }
      const offset = wedge[index];
      index += 1;
      if (offset < 0) {
        break;
      }
      x = baseX + offsetDirection * offset;
    }
    this.markTilemapDirty();
  }

  private fillBox(startX: number, startY: number, width: number, height: number): void {
    for (let y = startY; y < startY + height; y += 1) {
      for (let x = startX; x < startX + width; x += 1) {
        this.setTile(x, y, BATTLETRANSITION_BLACK);
      }
    }
    this.markTilemapDirty();
  }

  private blackOutRandomTile(): void {
    const maxAttempts = SCREEN_WIDTH_TILES * SCREEN_HEIGHT_TILES;
    let attempts = 0;
    while (attempts < maxAttempts) {
      attempts += 1;
      const y = this.rng.randrange(SCREEN_HEIGHT_TILES);
      const x = this.rng.randrange(SCREEN_WIDTH_TILES);
      if (this.tileMap[y][x] === BATTLETRANSITION_BLACK) {
        continue;
      }
      this.tileMap[y][x] = BATTLETRANSITION_BLACK;
      this.markTilemapDirty();
      return;
    }
  }

  private setTile(x: number, y: number, tileId: number): void {
    if (x < 0 || y < 0 || x >= SCREEN_WIDTH_TILES || y >= SCREEN_HEIGHT_TILES) {
      return;
    }
    this.tileMap[y][x] = tileId;
  }

  private requireScreen(): Surface {
    if (this.screen === null) {
      throw new Error("BattleTransitionManager requires an active screen");
    }
    return this.screen;
  }
}
