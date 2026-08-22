import { beginFlarePlotFrame, finishFlarePlotFrame } from "@pokecrystal/core/ui/flare-plot-renderer";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { type KeyEvent } from "@pokecrystal/core/input/buttons";
import { isCancelEvent, isConfirmEvent, isKeyDownEvent, isSelectEvent, isStartEvent } from "@pokecrystal/core/input/controls";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { HeadlessCanvas } from "@pokecrystal/core/ui/headless-canvas";
import {
  IntroSprite,
  BW_FADE_TABLE,
  BLACK_LIGHT_BLUE_FADE,
  BLACK_BLUE_FADE,
  FAST_FADE_PALETTES,
  SLOW_FADE_PALETTES,
  RGBColor,
} from "./intro-schemas";
import {
  loadFramesets,
  loadSpriteOamSets,
  loadSpriteObjectDefinitions,
} from "./asm-data";
import { IntroGraphics } from "./intro-graphics";
import {
  getTileIndexMode,
  getPaletteInversions,
  getTileShift,
  resolveGraphicName,
} from "./tilemap-defaults";
import type { TileIndexMode } from "./tilemap-defaults";
import {
  type TextSnapshotPayload,
} from "../../text-overlays";
import { buildIntroSequenceControlLines } from "../../control-lines";

// ASM reference: engine/movie/intro.asm::CrystalIntro

type IntroTilemapConfig = {
  gfxName: string;
  tileShift: number;
  tileIndexMode: TileIndexMode;
  paletteInversions: Record<number, boolean>;
  hasPaletteInversions: boolean;
};
type IntroMapData = ArrayLike<number> & { length: number };

const ensureIntroCanvasImageSource = (surface: InstanceType<typeof gameEngine.Surface>): CanvasImageSource => {
  const canvas = surface.canvas;
  if (canvas instanceof HeadlessCanvas) {
    return canvas as unknown as CanvasImageSource;
  }
  return canvas;
};

const OAM_XFLIP = 0x20;
const OAM_YFLIP = 0x40;
const OAM_PRIO = 0x80;
const OAM_ATTR_MASK = OAM_XFLIP | OAM_YFLIP | OAM_PRIO;

export class IntroSequence {
  static readonly SCREEN_WIDTH = 32;
  static readonly SCREEN_HEIGHT = 32;
  static readonly TILE_SIZE = 8;
  static readonly GBC_SCREEN_HEIGHT_PX = 144;
  static readonly FULL_TILE_COUNT = IntroSequence.SCREEN_WIDTH * IntroSequence.SCREEN_HEIGHT;
  static readonly VISIBLE_WIDTH = 20;
  static readonly VISIBLE_HEIGHT = 18;
  static readonly VISIBLE_TILE_COUNT = IntroSequence.VISIBLE_WIDTH * IntroSequence.VISIBLE_HEIGHT;
  static readonly SURFACE_WIDTH = IntroSequence.SCREEN_WIDTH * IntroSequence.TILE_SIZE;
  static readonly SURFACE_HEIGHT = IntroSequence.SCREEN_HEIGHT * IntroSequence.TILE_SIZE;
  static readonly VISIBLE_TILE_X_OFFSETS = (() => {
    const offsets = new Int16Array(IntroSequence.VISIBLE_TILE_COUNT);
    for (let i = 0; i < IntroSequence.VISIBLE_TILE_COUNT; i += 1) {
      offsets[i] = (i % IntroSequence.VISIBLE_WIDTH) * IntroSequence.TILE_SIZE;
    }
    return offsets;
  })();
  static readonly VISIBLE_TILE_Y_OFFSETS = (() => {
    const offsets = new Int16Array(IntroSequence.VISIBLE_TILE_COUNT);
    for (let i = 0; i < IntroSequence.VISIBLE_TILE_COUNT; i += 1) {
      offsets[i] = ((i / IntroSequence.VISIBLE_WIDTH) | 0) * IntroSequence.TILE_SIZE;
    }
    return offsets;
  })();
  static readonly TILE_X_OFFSETS = (() => {
    const offsets = new Int16Array(IntroSequence.FULL_TILE_COUNT);
    for (let i = 0; i < IntroSequence.FULL_TILE_COUNT; i += 1) {
      offsets[i] = (i % IntroSequence.SCREEN_WIDTH) * IntroSequence.TILE_SIZE;
    }
    return offsets;
  })();
  static readonly TILE_Y_OFFSETS = (() => {
    const offsets = new Int16Array(IntroSequence.FULL_TILE_COUNT);
    for (let i = 0; i < IntroSequence.FULL_TILE_COUNT; i += 1) {
      offsets[i] = ((i / IntroSequence.SCREEN_WIDTH) | 0) * IntroSequence.TILE_SIZE;
    }
    return offsets;
  })();
  private static readonly SCENE9_ATTR_WIDTH = 32;
  private static readonly SCENE9_ATTR_HEIGHT = 18;
  private static readonly SCENE9_ATTR_TILE_COUNT = IntroSequence.SCENE9_ATTR_WIDTH * IntroSequence.SCENE9_ATTR_HEIGHT;
  private static readonly SCENE_CLEAR_BG_PALS_DELAY_FRAMES = 2;
  private static readonly INTRO_CLEAR_BGPALS_SCENES = new Set<number>([
    0, // IntroScene1
    2, // IntroScene3
    4, // IntroScene5
    6, // IntroScene7
    10, // IntroScene11
    12, // IntroScene13
    14, // IntroScene15
    16, // IntroScene17
    18, // IntroScene19
  ]);
  private static readonly SCENE12_UNOWN_SOUNDS = [
    [0x00, "SFX_INTRO_UNOWN_3"],
    [0x20, "SFX_INTRO_UNOWN_2"],
    [0x40, "SFX_INTRO_UNOWN_1"],
    [0x60, "SFX_INTRO_UNOWN_2"],
    [0x80, "SFX_INTRO_UNOWN_3"],
    [0x90, "SFX_INTRO_UNOWN_2"],
    [0xa0, "SFX_INTRO_UNOWN_1"],
    [0xb0, "SFX_INTRO_UNOWN_2"],
  ] as const;
  // ASM: CrystalIntro_InitUnownAnim assigns framesets in this exact order:
  // UNOWN_4, UNOWN_3, UNOWN_1, UNOWN_2 — paired with direction angles 0x08, 0x18, 0x28, 0x38.
  private static readonly UNOWN_INIT_FRAMES = [
    "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_4",
    "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_3",
    "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1",
    "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_2",
  ] as const;
  // VAR1 holds each sprite's fixed direction angle (45°/135°/225°/315°, 64-step circle).
  // JUMPTABLE_INDEX is the growing distance, starting at 0 so all dots pulse from center.
  private static readonly UNOWN_INIT_AMPLITUDES = [0x08, 0x18, 0x28, 0x38] as const;
  private static readonly UNOWN_SWARM_SPAWN_COORDS = [
    [2, 2],
    [18, 2],
    [10, 4],
    [3, 7],
    [17, 7],
    [9, 9],
    [2, 12],
    [18, 12],
  ] as const;
  private static readonly SCENE_NAMES = [
    "unown_a",
    "unown_fade",
    "background_setup",
    "background_scroll",
    "unown_hi",
    "unown_pulse",
    "suicune_setup",
    "suicune_dash",
    "forest_hold",
    "grass_rustle",
    "unowns_tilemap",
    "unown_flash",
    "suicune_opening",
    "suicune_run",
    "suicune_jump",
    "suicune_rise",
    "suicune_close",
    "suicune_pan",
    "suicune_back",
    "unown_reveal",
    "color_swap",
    "sprite_clear",
    "transition",
    "palette_fade",
    "countdown",
    "crystal_unowns",
    "crystal_word_fade",
    "final_whoosh",
  ] as const;

  private readonly introGraphics: IntroGraphics;
  private readonly framesets = loadFramesets();
  private readonly oamSets = loadSpriteOamSets();
  private readonly spriteObjects = loadSpriteObjectDefinitions();

  private readonly bgMap0 = new gameEngine.Surface(
    IntroSequence.SCREEN_WIDTH * IntroSequence.TILE_SIZE,
    IntroSequence.SCREEN_HEIGHT * IntroSequence.TILE_SIZE
  );
  private readonly bgMap1 = new gameEngine.Surface(
    IntroSequence.SCREEN_WIDTH * IntroSequence.TILE_SIZE,
    IntroSequence.SCREEN_HEIGHT * IntroSequence.TILE_SIZE
  );
  private readonly windowLayer = new gameEngine.Surface(
    IntroSequence.SCREEN_WIDTH * IntroSequence.TILE_SIZE,
    IntroSequence.SCREEN_HEIGHT * IntroSequence.TILE_SIZE
  );
  private readonly scrolledBackgroundSurface = new gameEngine.Surface(
    IntroSequence.SURFACE_WIDTH,
    IntroSequence.SURFACE_HEIGHT
  );

  private alpha = 255;
  private sprites: IntroSprite[] = [];
  private activeTilemaps = new Map<InstanceType<typeof gameEngine.Surface>, string>();
  private activeTilemapTiles = new Map<InstanceType<typeof gameEngine.Surface>, IntroMapData>();
  private activeTilemapAttrmaps = new Map<
    InstanceType<typeof gameEngine.Surface>,
    IntroMapData
  >();
  private backgroundSurfaceCache: InstanceType<typeof gameEngine.Surface> | null = null;
  private backgroundCacheDirty = true;
  private tilemapConfigCache = new Map<string, IntroTilemapConfig>();
  private readonly renderSurfaceContexts = new Map<InstanceType<typeof gameEngine.Surface>, CanvasRenderingContext2D>();
  private readonly introScene9AttrmapCache = new Map<string, Uint8Array>();

  private readonly spriteAssetMap: Record<string, string> = {
    SPRITE_ANIM_OBJ_INTRO_UNOWN: "pulse",
    SPRITE_ANIM_OBJ_INTRO_UNOWN_F: "unown_back",
    SPRITE_ANIM_OBJ_INTRO_SUICUNE: "suicune_run",
    SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY: "suicune_back",
    SPRITE_ANIM_OBJ_INTRO_PICHU: "pichu_wooper",
    SPRITE_ANIM_OBJ_INTRO_WOOPER: "pichu_wooper",
  };

  private readonly scenes: Array<() => boolean> = [
    this.introScene1.bind(this),
    this.introScene2.bind(this),
    this.introScene3.bind(this),
    this.introScene4.bind(this),
    this.introScene5.bind(this),
    this.introScene6.bind(this),
    this.introScene7.bind(this),
    this.introScene8.bind(this),
    this.introScene9.bind(this),
    this.introScene10.bind(this),
    this.introScene11.bind(this),
    this.introScene12.bind(this),
    this.introScene13.bind(this),
    this.introScene14.bind(this),
    this.introScene15.bind(this),
    this.introScene16.bind(this),
    this.introScene17.bind(this),
    this.introScene18.bind(this),
    this.introScene19.bind(this),
    this.introScene20.bind(this),
    this.introScene21.bind(this),
    this.introScene22.bind(this),
    this.introScene23.bind(this),
    this.introScene24.bind(this),
    this.introScene25.bind(this),
    this.introScene26.bind(this),
    this.introScene27.bind(this),
    this.introScene28.bind(this),
  ];

  private jumptableIndex = 0;
  private sceneFrameCounter = 0;
  private sceneTimer = 0;
  private nextSceneFrameCounter: number | null = null;
  private sceneDelayFrames = 0;
  private readonly getSlotsDelay: () => number;
  private treeScrollOffset = 0;
  private grassScrollOffset = 0;
  private hSCX = 0;
  private hSCY = 0;
  private globalAnimXOffset = 0;
  private finished = false;

  private paletteSink?: (index: number, palette: RGBColor[]) => void;

  constructor(
    private readonly audioEngine: AudioEngine,
    opts?: {
      paletteSink?: (index: number, palette: RGBColor[]) => void;
      slotsDelay?: number;
      slotsDelaySource?: () => number;
    }
  ) {
    this.paletteSink = opts?.paletteSink;
    this.getSlotsDelay = opts?.slotsDelaySource ?? (() => opts?.slotsDelay ?? 0);
    this.introGraphics = new IntroGraphics();
    this.initializeIntro();
  }

  reset(): void {
    this.initializeIntro();
  }

  private initializeIntro(): void {
    this.initRamAddrs();
    // Clear all visual layers so no stale content from a previous intro
    // playthrough is visible on the first frame.
    this.clearTilemap();
    this.clearSprites();
    this.hSCX = 0;
    this.hSCY = 0;
    this.alpha = 255;
    this.treeScrollOffset = 0;
    this.grassScrollOffset = 0;
    this.globalAnimXOffset = 0;
    this.nextSceneFrameCounter = null;
    this.backgroundCacheDirty = true;
    this.backgroundSurfaceCache = null;
  }

  private initRamAddrs(): void {
    this.jumptableIndex = 0;
    this.sceneFrameCounter = 0;
    this.sceneTimer = 0;
    this.sceneDelayFrames = 0;
    this.finished = false;
    this.sprites = [];
  }

  update(): boolean {
    if (this.finished || this.jumptableIndex >= this.scenes.length) {
      this.finished = true;
      return true;
    }

    if (this.jumptableIndex & 0x80) {
      this.finished = true;
      return true;
    }

    if (this.sceneDelayFrames > 0) {
      this.sceneDelayFrames -= 1;
      if (this.sceneDelayFrames === 0) {
        this.nextScene();
        this.applySpriteAnimFunctions();
        this.updateSpriteAnimations();
      }
      return this.finished;
    }

    const sceneFinished = this.scenes[this.jumptableIndex]();
    const holdSpritePipeline = sceneFinished
      ? this.getSceneDelayFrames(this.jumptableIndex)
      : 0;

    if (holdSpritePipeline === 0) {
      this.applySpriteAnimFunctions();
      this.updateSpriteAnimations();
    }

    if (sceneFinished) {
      if (holdSpritePipeline > 0) {
        this.sceneDelayFrames = holdSpritePipeline;
      } else {
        this.nextScene();
      }
    } else if (this.nextSceneFrameCounter !== null) {
      this.sceneFrameCounter = this.nextSceneFrameCounter;
      this.nextSceneFrameCounter = null;
    } else {
      this.advanceSceneFrame();
    }

    return this.finished;
  }

  handleInput(event: KeyEvent): boolean {
    if (isKeyDownEvent(event)) {
      if (isStartEvent(event) || isSelectEvent(event) || isCancelEvent(event) || isConfirmEvent(event)) {
        this.audioEngine.playMusic("MUSIC_NONE", "intro");
        return true;
      }
    }
    return false;
  }

  draw(ctx: CanvasRenderingContext2D): void {
    const flareStart = beginFlarePlotFrame();
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
    this.drawLayer(ctx, this.bgMap0, this.hSCX, this.hSCY, this.alpha / 255);
    this.drawLayer(ctx, this.bgMap1, this.hSCX, this.hSCY, 1);
    ctx.drawImage(ensureIntroCanvasImageSource(this.windowLayer), 0, 0);

    for (const sprite of this.sprites) {
      this.drawSprite(ctx, sprite);
    }
    finishFlarePlotFrame(flareStart, "intro", ctx, 0, 0, 100, 30);
  }

  getDebugState(): {
    sceneIndex: number;
    sceneName: string;
    sceneFrameCounter: number;
    spriteCount: number;
    scrollX: number;
    scrollY: number;
    finished: boolean;
  } {
    const sceneIndex = this.jumptableIndex & 0x7f;
    const clampedIndex = Math.min(sceneIndex, IntroSequence.SCENE_NAMES.length - 1);
    return {
      sceneIndex,
      sceneName: IntroSequence.SCENE_NAMES[clampedIndex] ?? "complete",
      sceneFrameCounter: this.sceneFrameCounter,
      spriteCount: this.sprites.length,
      scrollX: this.hSCX,
      scrollY: this.hSCY,
      finished:
        this.finished ||
        (this.jumptableIndex & 0x80) !== 0 ||
        this.jumptableIndex >= this.scenes.length,
    };
  }

  getTextSnapshot(): TextSnapshotPayload {
    const debug = this.getDebugState();
    return {
      viewportLines: ["CRYSTAL INTRO", `SCENE: ${debug.sceneName}`],
      infoLines: [
        "STATE: intro",
        `SCENE INDEX: ${Math.min(debug.sceneIndex + 1, this.scenes.length)}/${this.scenes.length}`,
        `SCENE FRAME: ${debug.sceneFrameCounter}`,
        `SPRITES: ${debug.spriteCount}`,
        `SCROLL: x=${debug.scrollX} y=${debug.scrollY}`,
        `FINISHED: ${debug.finished ? "yes" : "no"}`,
        ...buildIntroSequenceControlLines(debug.finished),
      ],
      viewportTitle: "Intro",
      infoTitle: "Intro",
      menuLines: null,
      promptLines: null,
      dialogueLines: null,
    };
  }

  private drawLayer(
    ctx: CanvasRenderingContext2D,
    surface: InstanceType<typeof gameEngine.Surface>,
    scrollX: number,
    scrollY: number,
    alpha: number
  ): void {
    if (!this.shouldWrapSurface(surface)) {
      const previousAlpha = ctx.globalAlpha;
      ctx.globalAlpha = alpha;
      ctx.drawImage(ensureIntroCanvasImageSource(surface), -scrollX, -scrollY);
      ctx.globalAlpha = previousAlpha;
      return;
    }

    this.drawWrappedLayer(ctx, surface, scrollX, scrollY, alpha);
  }

  private shouldWrapSurface(surface: InstanceType<typeof gameEngine.Surface>): boolean {
    return this.activeTilemaps.has(surface);
  }

  private drawWrappedLayer(
    ctx: CanvasRenderingContext2D,
    surface: InstanceType<typeof gameEngine.Surface>,
    scrollX: number,
    scrollY: number,
    alpha: number
  ): void {
    const source = ensureIntroCanvasImageSource(surface);
    const xOffsets = IntroSequence.computeWrappedBlitOffsets(
      scrollX,
      IntroSequence.SURFACE_WIDTH,
      ctx.canvas.width
    );
    const yOffsets = IntroSequence.computeWrappedBlitOffsets(
      scrollY,
      IntroSequence.SURFACE_HEIGHT,
      ctx.canvas.height
    );
    const previousAlpha = ctx.globalAlpha;
    ctx.globalAlpha = alpha;
    for (const x of xOffsets) {
      for (const y of yOffsets) {
        ctx.drawImage(source, x, y);
      }
    }
    ctx.globalAlpha = previousAlpha;
  }

  private updateSpriteAnimations(): void {
    const nextSprites: IntroSprite[] = [];
    for (const sprite of this.sprites) {
      if (sprite.start_delay > 0) {
        sprite.start_delay -= 1;
        nextSprites.push(sprite);
        continue;
      }

      const frameset = this.framesets[sprite.frameset_name ?? ""];
      if (!frameset || !frameset.steps.length) {
        nextSprites.push(sprite);
        continue;
      }

      let removed = false;
      let stepIndex = sprite.frameset_step;
      const totalSteps = frameset.steps.length;

      while (true) {
        if (stepIndex < 0 || sprite.frame_timer === 0) {
          stepIndex += 1;
          if (stepIndex < 0) {
            stepIndex = 0;
          } else if (stepIndex >= totalSteps) {
            stepIndex = totalSteps - 1;
          }
        } else {
          sprite.frame_timer -= 1;
          break;
        }

        const step = frameset.steps[stepIndex];
        if (step.command === "frame") {
          sprite.current_oam_set = step.oam_set ?? null;
          sprite.attr_flags = step.attr_flags ?? 0;
          sprite.frame_timer = Math.max(0, step.duration - 1);
          break;
        }

        if (step.command === "wait") {
          sprite.frame_timer = Math.max(0, step.duration - 1);
          break;
        } else if (step.command === "restart") {
          stepIndex = -1;
          sprite.frame_timer = 0;
          continue;
        } else if (step.command === "delete") {
          removed = true;
          break;
        } else if (step.command === "end") {
          stepIndex -= 2;
          sprite.frame_timer = 0;
          continue;
        } else {
          stepIndex += 1;
          if (stepIndex >= totalSteps) {
            stepIndex = totalSteps - 1;
          }
          sprite.frame_timer = 0;
          continue;
        }
      }

      sprite.frameset_step = stepIndex;
      if (!removed) {
        nextSprites.push(sprite);
      }
    }

    this.sprites = nextSprites;
  }

  private static sine(angle: number, amplitude: number): number {
    const radians = (angle & 0x3f) * Math.PI / 32;
    return Math.round(Math.sin(radians) * amplitude);
  }

  private static cosine(angle: number, amplitude: number): number {
    const radians = (angle & 0x3f) * Math.PI / 32;
    return Math.round(Math.cos(radians) * amplitude);
  }

  private static tickSceneFrame(counter: number): number {
    return (counter + 1) & 0xff;
  }

  private static computeWrappedBlitOffsets(scroll: number, surfaceSize: number, viewportSize: number): number[] {
    if (surfaceSize <= 0 || viewportSize <= 0) {
      return [0];
    }
    const normalizedScroll = ((scroll % surfaceSize) + surfaceSize) % surfaceSize;
    const primaryOffset = normalizedScroll === 0 ? 0 : -normalizedScroll;
    const offsets = [primaryOffset];
    if (primaryOffset + surfaceSize < viewportSize) {
      offsets.push(primaryOffset + surfaceSize);
    }
    return offsets;
  }

  private advanceSceneFrame(): void {
    this.sceneFrameCounter = IntroSequence.tickSceneFrame(this.sceneFrameCounter);
  }

  private static signedByte(value: number): number {
    const v = value & 0xff;
    return v & 0x80 ? v - 0x100 : v;
  }

  private static applyFrameFlip(offset: number, flip: boolean): number {
    return flip ? -8 - offset : offset;
  }

  private applySpriteAnimFunctions(): void {
    for (const sprite of this.sprites) {
      const func = sprite.anim_function;
      if (func === "SPRITE_ANIM_FUNC_INTRO_SUICUNE") {
        if (this.sceneTimer === 0) {
          sprite.y_offset = 0;
          continue;
        }
        sprite.var2 = (sprite.var2 + 2) & 0xff;
        const angle = (~sprite.var2 + 1) & 0xff;
        sprite.y_offset = IntroSequence.sine(angle, 32);
        sprite.x_offset = 0;
        if (sprite.frameset_name !== "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2") {
          sprite.frameset_name = "SPRITE_ANIM_FRAMESET_INTRO_SUICUNE_2";
          sprite.frameset_step = -1;
          sprite.frame_timer = 0;
          sprite.current_oam_set = null;
        }
      } else if (func === "SPRITE_ANIM_FUNC_INTRO_PICHU_WOOPER") {
        if (sprite.var1 < 20) {
          sprite.var1 = (sprite.var1 + 2) & 0xff;
        }
        const angle = (~sprite.var1 + 1) & 0xff;
        sprite.y_offset = IntroSequence.sine(angle, 32);
        sprite.x_offset = 0;
      } else if (func === "SPRITE_ANIM_FUNC_INTRO_UNOWN") {
        // ASM: engine/sprite_anims/functions.asm::SpriteAnimFunc_IntroUnown
        // VAR1 is the fixed direction angle; JUMPTABLE_INDEX is the growing distance.
        // Y = JUMPTABLE_INDEX * sin(VAR1 * pi/32)
        // X = JUMPTABLE_INDEX * cos(VAR1 * pi/32)
        const direction = sprite.var1 & 0xff;
        const distance = sprite.jumptable_index & 0xff;
        sprite.y_offset = IntroSequence.sine(direction, distance);
        sprite.x_offset = IntroSequence.cosine(direction, distance);
        sprite.jumptable_index = (sprite.jumptable_index + 3) & 0xff;
      } else if (func === "SPRITE_ANIM_FUNC_INTRO_UNOWN_F") {
        if ((this.getSlotsDelay() & 0xff) === 0x40) {
          if (sprite.frameset_name !== "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F_2") {
            sprite.frameset_name = "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_F_2";
            sprite.frameset_step = -1;
            sprite.frame_timer = 0;
            sprite.current_oam_set = null;
          }
        }
      } else if (func === "SPRITE_ANIM_FUNC_INTRO_SUICUNE_AWAY") {
        sprite.y += 16;
        sprite.x_offset = 0;
      }
    }
  }

  private drawSprite(ctx: CanvasRenderingContext2D, sprite: IntroSprite): void {
    if (sprite.start_delay > 0) {
      return;
    }

    let oamSetName = sprite.current_oam_set;
    if (!oamSetName) {
      const frameset = this.framesets[sprite.frameset_name ?? ""];
      const frameIndex = Math.max(0, sprite.frameset_step);
      if (frameset && frameIndex < frameset.steps.length) {
        const step = frameset.steps[frameIndex];
        if (step.command === "frame") {
          oamSetName = step.oam_set ?? null;
          sprite.current_oam_set = oamSetName;
          sprite.attr_flags = step.attr_flags ?? 0;
        }
      }
    }

    if (!oamSetName) {
      return;
    }
    const oamSet = this.oamSets[oamSetName];
    if (!oamSet) {
      return;
    }

    const baseGraphic = this.spriteAssetMap[sprite.object_name ?? ""] ?? sprite.gfx_name;
    const frameFlags = sprite.attr_flags & OAM_ATTR_MASK;

    for (const piece of oamSet.pieces) {
      const baseAttr = piece.attributes | sprite.oam_attr;
      const flippedAttr = (baseAttr ^ frameFlags) & OAM_ATTR_MASK;
      const attr = (baseAttr & ~OAM_ATTR_MASK) | flippedAttr;
      const paletteIdx = attr & 0x7;
      const tileIndex = piece.tile + oamSet.tile_offset;
      const tileSurface = this.introGraphics.getTile(
        baseGraphic,
        tileIndex,
        paletteIdx,
        attr,
        true
      );
      if (!tileSurface) {
        continue;
      }

      const offsetX = IntroSequence.applyFrameFlip(piece.x, (frameFlags & OAM_XFLIP) !== 0);
      const offsetY = IntroSequence.applyFrameFlip(piece.y, (frameFlags & OAM_YFLIP) !== 0);
      const globalX = IntroSequence.signedByte(this.globalAnimXOffset);
      const drawX = sprite.x + sprite.x_offset + globalX + offsetX - 8;
      const drawY = sprite.y + sprite.y_offset + offsetY - 16;
      ctx.drawImage(ensureIntroCanvasImageSource(tileSurface), drawX, drawY);
    }
  }

  private nextScene(): void {
    this.jumptableIndex += 1;
    this.sceneFrameCounter = this.nextSceneFrameCounter ?? 0;
    this.nextSceneFrameCounter = null;
    this.sceneTimer = 0;
    this.sceneDelayFrames = 0;
    if (this.jumptableIndex >= this.scenes.length) {
      this.finished = true;
    }
  }

  private getSceneDelayFrames(index: number): number {
    if (IntroSequence.INTRO_CLEAR_BGPALS_SCENES.has(index)) {
      return IntroSequence.SCENE_CLEAR_BG_PALS_DELAY_FRAMES;
    }
    if (index === 8) {
      return 6;
    }
    if (index === 20) {
      return 3;
    }
    return 0;
  }

  private drawTilemap(name: string, surface: InstanceType<typeof gameEngine.Surface>): void {
    this.renderTilemap(name, surface, true);
  }

  private getTilemapConfig(name: string): IntroTilemapConfig {
    const cached = this.tilemapConfigCache.get(name);
    if (cached) {
      return cached;
    }

    const paletteInversions = getPaletteInversions(name);
    const config: IntroTilemapConfig = {
      gfxName: resolveGraphicName(name),
      tileShift: getTileShift(name),
      tileIndexMode: getTileIndexMode(name),
      paletteInversions,
      hasPaletteInversions: Object.keys(paletteInversions).length > 0,
    };
    this.tilemapConfigCache.set(name, config);
    return config;
  }

  private renderTilemap(
    name: string,
    surface: InstanceType<typeof gameEngine.Surface>,
    store: boolean,
    tilemapOverride: IntroMapData | null = null,
    attrmapOverride: IntroMapData | null = null,
    tileCount = IntroSequence.FULL_TILE_COUNT,
    tileTransform?: (tileIdx: number) => number
  ): void {
    const config = this.getTilemapConfig(name);
    const tilemap = tilemapOverride ?? this.introGraphics.tilemaps[name];
    const baseAttrmap = this.introGraphics.attrmaps[name];
    const attrmap = attrmapOverride ?? baseAttrmap;
    if (!tilemap || !attrmap) {
      return;
    }

    if (store && config.hasPaletteInversions) {
      this.applyTilemapPaletteDefaults(config.gfxName, config.paletteInversions);
    }

    this.clearSurface(surface);

    const renderTileCount = Math.max(0, Math.min(tileCount, IntroSequence.FULL_TILE_COUNT));
    const useVisibleOffsets = renderTileCount === IntroSequence.VISIBLE_TILE_COUNT;
    const tileXOffsets = useVisibleOffsets
      ? IntroSequence.VISIBLE_TILE_X_OFFSETS
      : IntroSequence.TILE_X_OFFSETS;
    const tileYOffsets = useVisibleOffsets
      ? IntroSequence.VISIBLE_TILE_Y_OFFSETS
      : IntroSequence.TILE_Y_OFFSETS;

    for (let i = 0; i < renderTileCount; i += 1) {
      const sourceTileIdx = tilemap[i] ?? 0;
      const tileIdx = tileTransform ? tileTransform(sourceTileIdx) : sourceTileIdx;
      const attr = attrmap[i] ?? 0;
      const paletteIdx = attr & 0x7;
      let tileGfx = config.gfxName;
      let tileBase = config.tileShift;
      let tileIndexMode = config.tileIndexMode;
      let paletteNameOverride: string | undefined;
      if (name === "suicune_back" && tileIdx >= 0x80) {
        tileGfx = "unowns";
        tileBase = 0x80;
        tileIndexMode = "offset";
        paletteNameOverride = "suicune";
      }

      const tileSurface = this.introGraphics.getTile(
        tileGfx,
        tileIdx & 0xff,
        paletteIdx,
        attr,
        false,
        tileBase,
        paletteNameOverride,
        tileIndexMode
      );
      if (tileSurface) {
        surface.blit(tileSurface, [
          tileXOffsets[i],
          tileYOffsets[i],
        ]);
      }
    }

    if (store) {
      this.activeTilemaps.set(surface, name);
      this.activeTilemapTiles.set(surface, tilemap);
      this.activeTilemapAttrmaps.set(surface, attrmap);
      if (name === "background") {
        this.backgroundCacheDirty = true;
      }
    }
  }

  private redrawSurface(surface: InstanceType<typeof gameEngine.Surface>): void {
    const name = this.activeTilemaps.get(surface);
    if (!name) {
      return;
    }
    const tilemapOverride = this.activeTilemapTiles.get(surface) ?? null;
    const attrmapOverride = this.activeTilemapAttrmaps.get(surface) ?? null;
    this.renderTilemap(name, surface, false, tilemapOverride, attrmapOverride);
  }

  private applyTilemapPaletteDefaults(
    paletteName: string,
    paletteInversions: Record<number, boolean>
  ): void {
    if (!paletteName) {
      return;
    }

    const baseBgPalettes = this.introGraphics.palettes[paletteName];
    const baseObjPalettes = this.introGraphics.objPalettes[paletteName];

    const bgOverrides = baseBgPalettes
      ? (this.introGraphics.paletteOverrides[paletteName] ||= {})
      : this.introGraphics.paletteOverrides[paletteName];
    const objOverrides = baseObjPalettes
      ? (this.introGraphics.objPaletteOverrides[paletteName] ||= {})
      : this.introGraphics.objPaletteOverrides[paletteName];

    for (const paletteIdxRaw in paletteInversions) {
      const paletteIdx = Number(paletteIdxRaw);
      const inverted = paletteInversions[paletteIdx];
      const applyInvert = (base: RGBColor[][] | undefined, overrides?: Record<number, RGBColor[]>) => {
        if (!base || !overrides) {
          return;
        }
        if (paletteIdx >= base.length) {
          delete overrides[paletteIdx];
          return;
        }
        if (inverted) {
          overrides[paletteIdx] = [...base[paletteIdx]].reverse();
        } else {
          delete overrides[paletteIdx];
        }
      };

      if (inverted) {
        applyInvert(baseBgPalettes, bgOverrides);
        applyInvert(baseObjPalettes, objOverrides);
      } else {
        if (bgOverrides) {
          delete bgOverrides[paletteIdx];
        }
        if (objOverrides) {
          delete objOverrides[paletteIdx];
        }
      }
    }

    if (bgOverrides && Object.keys(bgOverrides).length === 0) {
      delete this.introGraphics.paletteOverrides[paletteName];
    }
    if (objOverrides && Object.keys(objOverrides).length === 0) {
      delete this.introGraphics.objPaletteOverrides[paletteName];
    }
  }

  private coloredSuicuneFrameSwap(): void {
    const name = this.activeTilemaps.get(this.bgMap0);
    if (!name) {
      return;
    }

    const tilemap = this.activeTilemapTiles.get(this.bgMap0) ?? this.introGraphics.tilemaps[name];
    if (!tilemap) {
      return;
    }

    this.renderTilemap(
      name,
      this.bgMap0,
      true,
      tilemap,
      this.activeTilemapAttrmaps.get(this.bgMap0) ?? null,
      IntroSequence.FULL_TILE_COUNT,
      (tileIdx) => (tileIdx !== 0 && tileIdx < 0x80 ? tileIdx ^ 0x08 : tileIdx)
    );
  }

  private applyFade(palette: RGBColor[], fadeLevel: number): RGBColor[] {
    return palette.map(([r, g, b]) => [
      Math.floor((r * fadeLevel) / 32),
      Math.floor((g * fadeLevel) / 32),
      Math.floor((b * fadeLevel) / 32),
    ]);
  }

  private setPalette(index: number, palette: RGBColor[]): void {
    if (this.paletteSink) {
      this.paletteSink(index, palette);
    }
  }

  private unownFade(paletteIdx: number): void {
    let timer = this.sceneTimer & 0x3f;
    if (timer > 0x1f) {
      timer = 0x3f - timer;
    }

    const bwColor = BW_FADE_TABLE[timer];
    const lightBlueColor = BLACK_LIGHT_BLUE_FADE[timer];
    const blueColor = BLACK_BLUE_FADE[timer];

    for (let i = 0; i < 8; i++) {
        if (i === paletteIdx) {
            const basePalette = [...(this.introGraphics.palettes.unowns?.[paletteIdx] ?? [])];
            if (basePalette.length >= 4) {
              basePalette[1] = bwColor;
              basePalette[2] = lightBlueColor;
              basePalette[3] = blueColor;
              this.setPalette(paletteIdx, basePalette);
              this.introGraphics.setPaletteOverride("unowns", paletteIdx, basePalette);
            }
        } else {
            const blackPalette: RGBColor[] = [[0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0]];
            this.setPalette(i, blackPalette);
            this.introGraphics.setPaletteOverride("unowns", i, blackPalette);
        }
    }

    [this.bgMap0, this.bgMap1].forEach((surface) => {
      const active = this.activeTilemaps.get(surface);
      if (active && ["unown_a", "unown_hi", "unowns"].includes(active)) {
        this.redrawSurface(surface);
      }
    });
  }

  private initUnownAnim(x: number, y: number): void {
    IntroSequence.UNOWN_INIT_FRAMES.forEach((framesetName, index) => {
      const sprite = this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_UNOWN", x, y);
      sprite.frameset_name = framesetName;
      sprite.start_delay = 0;
      // var1 holds the fixed direction angle for this sprite's radial pulse.
      sprite.var1 = IntroSequence.UNOWN_INIT_AMPLITUDES[index];
      // Start at distance 0 so all four dots pulse outward from center.
      sprite.jumptable_index = 0;
    });
  }

  private initUnownSwarm(): void {
    IntroSequence.UNOWN_SWARM_SPAWN_COORDS.forEach(([x, y]) => {
      const sprite = this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_UNOWN", x * 8, y * 8);
      sprite.frameset_name = "SPRITE_ANIM_FRAMESET_INTRO_UNOWN_1";
      sprite.var1 = 0x18;
    });
  }

  private getSurfaceContext(surface: InstanceType<typeof gameEngine.Surface>): CanvasRenderingContext2D {
    const cached = this.renderSurfaceContexts.get(surface);
    if (cached) {
      return cached;
    }
    const context = surface.canvas.getContext("2d") as CanvasRenderingContext2D;
    if (!context) {
      throw new Error("Unable to access intro layer 2D context.");
    }
    this.renderSurfaceContexts.set(surface, context);
    return context;
  }

  private clearSurface(surface: InstanceType<typeof gameEngine.Surface>): void {
    const context = this.getSurfaceContext(surface);
    if (typeof context.clearRect === "function") {
      context.clearRect(0, 0, surface.get_width(), surface.get_height());
      return;
    }
    surface.fill([0, 0, 0, 0]);
  }

  private getScene9AdjustedAttrmap(name: string, baseAttrmap: IntroMapData): IntroMapData {
    const cached = this.introScene9AttrmapCache.get(name);
    if (cached) {
      return cached;
    }

    const adjusted = new Uint8Array(IntroSequence.SCENE9_ATTR_TILE_COUNT);
    for (let i = 0; i < adjusted.length; i += 1) {
      adjusted[i] = baseAttrmap[i] ?? 0;
    }

    for (let row = 0; row < IntroSequence.SCENE9_ATTR_HEIGHT; row += 1) {
      const palette = row < 12 ? 1 : row < 15 ? 2 : 3;
      const rowStart = row * IntroSequence.SCENE9_ATTR_WIDTH;
      const rowEnd = rowStart + IntroSequence.SCENE9_ATTR_WIDTH;
      for (let i = rowStart; i < rowEnd; i += 1) {
        adjusted[i] = (adjusted[i] & 0xf8) | (palette & 0x7);
      }
    }

    this.introScene9AttrmapCache.set(name, adjusted);
    return adjusted;
  }

  private clearSprites(): void {
    this.sprites = [];
  }

  private clearTilemap(): void {
    this.clearSurface(this.bgMap0);
    this.clearSurface(this.bgMap1);
    this.clearSurface(this.windowLayer);
    this.clearSurface(this.scrolledBackgroundSurface);
    this.alpha = 255;
    this.activeTilemaps = new Map();
    this.activeTilemapTiles = new Map();
    this.activeTilemapAttrmaps = new Map();
    this.introGraphics.clearPaletteOverrides();
  }

  private clearBgPalettes(): void {
    const blackPalette: RGBColor[] = [[0, 0, 0], [0, 0, 0], [0, 0, 0], [0, 0, 0]];
    const activePaletteNames = this.getActiveDisplayedBgPaletteNames();
    for (let palIdx = 0; palIdx < 8; palIdx += 1) {
      this.setPalette(palIdx, blackPalette);
      for (const paletteName of activePaletteNames) {
        this.introGraphics.setPaletteOverride(paletteName, palIdx, [...blackPalette]);
      }
    }
    this.redrawDisplayedBgLayers();
  }

  private getDisplayedBgSurfaces(): Array<InstanceType<typeof gameEngine.Surface>> {
    return [this.bgMap0, this.bgMap1, this.windowLayer];
  }

  private getActiveDisplayedBgPaletteNames(): string[] {
    const names = new Set<string>();
    for (const surface of this.getDisplayedBgSurfaces()) {
      const active = this.activeTilemaps.get(surface);
      if (active) {
        const paletteName = this.introGraphics.getResolvedPaletteName(
          resolveGraphicName(active)
        );
        if (paletteName) {
          names.add(paletteName);
        }
      }
    }
    return Array.from(names);
  }

  private redrawDisplayedBgLayers(): void {
    for (const surface of this.getDisplayedBgSurfaces()) {
      this.redrawSurface(surface);
    }
  }

  private spawnSprite(objectName: string, x: number, y: number): IntroSprite {
    const definition = this.spriteObjects[objectName];
    if (!definition) {
      throw new Error(`Unknown intro sprite object: ${objectName}`);
    }
    const animIdMap: Record<string, number> = {
      SPRITE_ANIM_OBJ_INTRO_SUICUNE: 26,
      SPRITE_ANIM_OBJ_INTRO_PICHU: 27,
      SPRITE_ANIM_OBJ_INTRO_WOOPER: 28,
      SPRITE_ANIM_OBJ_INTRO_UNOWN: 29,
      SPRITE_ANIM_OBJ_INTRO_UNOWN_F: 30,
      SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY: 31,
    };
    const sprite = new IntroSprite(x, y, animIdMap[objectName] ?? 0);
    sprite.object_name = objectName;
    sprite.frameset_name = definition.frameset;
    sprite.anim_function = definition.function;
    sprite.gfx_name = this.spriteAssetMap[objectName] ?? sprite.gfx_name;
    sprite.jumptable_index = 0;
    sprite.frame_timer = 0;
    sprite.frameset_step = -1;
    sprite.start_delay = 0;
    sprite.current_oam_set = null;
    this.sprites.push(sprite);
    return sprite;
  }

  private drawBackgroundWithScroll(scrollValues: number[]): void {
    this.clearSurface(this.bgMap0);
    const backgroundSurface = this.getBackgroundSource();
    const ctx = this.getSurfaceContext(this.bgMap0);
    const backgroundSource =
      backgroundSurface.getCanvasImageSource?.() ??
      (backgroundSurface.canvas as unknown as CanvasImageSource);
    for (let i = 0; i < IntroSequence.GBC_SCREEN_HEIGHT_PX; i += 1) {
      const scroll = (scrollValues[i] ?? 0) & 0xff;
      ctx.drawImage(
        backgroundSource,
        0,
        i,
        IntroSequence.SURFACE_WIDTH,
        1,
        -scroll,
        i,
        IntroSequence.SURFACE_WIDTH,
        1
      );
    }

    this.clearSurface(this.scrolledBackgroundSurface);
    this.scrolledBackgroundSurface.blit(this.bgMap0, [-this.treeScrollOffset, 0]);
    const activeName = this.activeTilemaps.get(this.bgMap0);
    const tiles = this.activeTilemapTiles.get(this.bgMap0) ?? null;
    const attrs = this.activeTilemapAttrmaps.get(this.bgMap0) ?? null;
    this.activeTilemaps.delete(this.bgMap0);
    this.activeTilemapTiles.delete(this.bgMap0);
    this.activeTilemapAttrmaps.delete(this.bgMap0);
    if (activeName) {
      this.activeTilemaps.set(this.scrolledBackgroundSurface, activeName);
    }
    if (tiles) {
      this.activeTilemapTiles.set(this.scrolledBackgroundSurface, tiles);
    }
    if (attrs) {
      this.activeTilemapAttrmaps.set(this.scrolledBackgroundSurface, attrs);
    }
  }

  private getBackgroundSource(): InstanceType<typeof gameEngine.Surface> {
    if (!this.backgroundSurfaceCache || this.backgroundCacheDirty) {
      const surface = new gameEngine.Surface(
        IntroSequence.SURFACE_WIDTH,
        IntroSequence.SURFACE_HEIGHT
      );
      this.renderTilemap("background", surface, false);
      this.backgroundSurfaceCache = surface;
      this.backgroundCacheDirty = false;
    }
    return this.backgroundSurfaceCache;
  }

  private perspectiveScrollBg(): void {
    this.grassScrollOffset = (this.grassScrollOffset + 2) & 0xff;
    if ((this.sceneFrameCounter & 1) === 0) {
      this.treeScrollOffset = (this.treeScrollOffset + 1) & 0xff;
    }

    const source = this.getBackgroundSource();
    const sourceImage = ensureIntroCanvasImageSource(source);
    this.clearSurface(this.bgMap0);
    const ctx = this.getSurfaceContext(this.bgMap0);

    for (let y = 0; y < IntroSequence.GBC_SCREEN_HEIGHT_PX; y += 1) {
      const scroll = y < 0x5f ? this.treeScrollOffset : this.grassScrollOffset;
      const offset = scroll & 0xff;
      if (offset === 0) {
        ctx.drawImage(
          sourceImage,
          0,
          y,
          IntroSequence.SURFACE_WIDTH,
          1,
          0,
          y,
          IntroSequence.SURFACE_WIDTH,
          1
        );
      } else {
        const leftWidth = IntroSequence.SURFACE_WIDTH - offset;
        ctx.drawImage(
          sourceImage,
          offset,
          y,
          leftWidth,
          1,
          0,
          y,
          leftWidth,
          1
        );
        ctx.drawImage(
          sourceImage,
          0,
          y,
          offset,
          1,
          leftWidth,
          y,
          offset,
          1
        );
      }
    }

    this.activeTilemaps.set(this.bgMap0, "background");
  }

  private rustleGrass(): void {
    if (this.sceneFrameCounter >= 0x24) {
      return;
    }

    const frame = (this.sceneFrameCounter & 0x0c) >> 2;
    const grassGfx = ["grass1", "grass2", "grass3", "grass2"][frame];
    for (let i = 0; i < 4; i++) {
      const tile = this.introGraphics.getTile(grassGfx, i, 2);
      if (tile) {
        this.bgMap1.blit(tile, [
          9 * IntroSequence.TILE_SIZE + i * IntroSequence.TILE_SIZE,
          12 * IntroSequence.TILE_SIZE,
        ]);
      }
    }
  }

  private appearUnown(paletteIdx: number): void {
    const paletteSet = paletteIdx === 0 ? "unown_1" : "unown_2";
    const targetIndex = this.sceneTimer & 0x7;
    const palettes = this.introGraphics.palettes[paletteSet] ?? [];
    if (palettes.length) {
      const palette = palettes[0];
      if (this.introGraphics.palettes.unowns) {
        this.introGraphics.setPaletteOverride("unowns", targetIndex, palette);
      }
      // ASM hardware palette write at index N affects all tiles with that attrmap palette index,
      // spanning both the Unowns GFX (vTiles1) and SuicuneBack GFX (vTiles2) tile sets.
      if (this.introGraphics.palettes.suicune) {
        this.introGraphics.setPaletteOverride("suicune", targetIndex, palette);
      }
      this.setPalette(targetIndex, palette);
      this.redrawSurface(this.bgMap0);
    }
  }

  private fadeUnownWordPals(fadeLevel: number): void {
    // ASM: c = wIntroSceneTimer * 2 is a byte offset into a 2-byte-per-color table,
    // so the color index is just wIntroSceneTimer (not * 2).
    const timerIndex = this.sceneTimer;
    const fastRgb = FAST_FADE_PALETTES[timerIndex] ?? [0, 0, 0];
    const slowRgb = SLOW_FADE_PALETTES[timerIndex] ?? [0, 0, 0];
    const palIdx = fadeLevel;
    const palettes = this.introGraphics.palettes.crystal_unowns;
    if (!palettes || palIdx >= palettes.length) {
      return;
    }
    const basePalette = [...palettes[palIdx]];
    basePalette[2] = fastRgb;
    basePalette[3] = slowRgb;
    this.introGraphics.setPaletteOverride("crystal_unowns", palIdx, basePalette);
    this.setPalette(palIdx, basePalette);

    [this.bgMap0, this.bgMap1].forEach((surface) => {
      const active = this.activeTilemaps.get(surface);
      if (active === "crystal_unowns") {
        this.redrawSurface(surface);
      }
    });
  }

  private applyScene24PaletteFade(fadeIndex: number): void {
    const fadePalettes = this.introGraphics.palettes.fade;
    if (!fadePalettes?.length) {
      return;
    }
    const idx = Math.min(fadeIndex, fadePalettes.length - 1);
    const palette = fadePalettes[idx];
    const activePaletteNames = this.getActiveDisplayedBgPaletteNames();

    for (let palIdx = 0; palIdx < 8; palIdx++) {
      this.setPalette(palIdx, palette);
      activePaletteNames.forEach((paletteName) => {
        if (paletteName) {
          this.introGraphics.setPaletteOverride(paletteName, palIdx, [...palette]);
        }
      });
    }

    this.redrawDisplayedBgLayers();
  }

  // Scene implementations -----------------------------------------------------
  private introScene1(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("unown_a", this.bgMap0);
    this.sceneFrameCounter = 0;
    this.sceneTimer = 0;
    return true;
  }

  private introScene2(): boolean {
    const frame = this.sceneFrameCounter;
    if (frame >= 0x80) {
      return true;
    }
    if (frame === 0x60) {
      this.initUnownAnim(11 * 8, 11 * 8);
      this.audioEngine.playSound("SFX_INTRO_UNOWN_1");
    }
    this.sceneTimer = frame;
    this.unownFade(0);
    return false;
  }

  private introScene3(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("background", this.bgMap0);
    this.sceneFrameCounter = 0;
    return true;
  }

  private introScene4(): boolean {
    this.perspectiveScrollBg();
    if (this.sceneFrameCounter === 0x80) {
      return true;
    }
    return false;
  }

  private introScene5(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("unown_hi", this.bgMap0);
    this.sceneFrameCounter = 0;
    return true;
  }

  private introScene6(): boolean {
    const frame = this.sceneFrameCounter;
    if (frame >= 0x80) {
      return true;
    }
    if (frame === 0x20) {
      // Crystal intro scene 6's first visible pulse lands on the upper-right Unown
      // in renderer screen-space.
      this.initUnownAnim(15 * 8, 7 * 8);
      this.audioEngine.playSound("SFX_INTRO_UNOWN_2");
    } else if (frame === 0x60) {
      // ASM tilemap parity: palette 1 in `unown_hi` only lights the lower-left Unown
      // at tiles x=3..6, y=11..13, so the third pulse must spawn over that glyph.
      this.initUnownAnim(5 * 8, 14 * 8);
      this.audioEngine.playSound("SFX_INTRO_UNOWN_1");
    }

    this.sceneTimer = frame;
    this.unownFade(frame >= 0x40 ? 1 : 0);
    return false;
  }

  private introScene7(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("background", this.bgMap0);
    // ASM: depixel 13, 27, 4, 0 => x=27, y=13
    this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_SUICUNE", 27 * 8, 13 * 8 + 4);
    this.globalAnimXOffset = 0xf0;
    this.sceneFrameCounter = 0;
    this.sceneTimer = 0;
    return true;
  }

  private introScene8(): boolean {
    const frame = this.sceneFrameCounter;
    if (frame < 0x40) {
      this.perspectiveScrollBg();
    } else {
      if (frame === 0x40) {
        this.audioEngine.playSound("SFX_INTRO_SUICUNE_3");
      }
      if (this.globalAnimXOffset === 0) {
        this.audioEngine.playSound("SFX_INTRO_SUICUNE_2");
        this.clearSprites();
        return true;
      }
      this.globalAnimXOffset = (this.globalAnimXOffset - 8) & 0xff;
    }
    return false;
  }

  private introScene9(): boolean {
    this.clearSprites();
    const activeName = this.activeTilemaps.get(this.bgMap0);
    if (activeName) {
      const tilemap = this.activeTilemapTiles.get(this.bgMap0) ?? this.introGraphics.tilemaps[activeName];
      const baseAttrmap = this.introGraphics.attrmaps[activeName];
      if (tilemap && baseAttrmap) {
        const adjustedAttrmap = this.getScene9AdjustedAttrmap(activeName, baseAttrmap);
        this.renderTilemap(activeName, this.bgMap0, true, tilemap, adjustedAttrmap, IntroSequence.SCENE9_ATTR_TILE_COUNT);
      }
    }
    this.hSCX = this.treeScrollOffset & 0xff;
    this.sceneTimer = 0;
    this.globalAnimXOffset = 0;
    return true;
  }

  private introScene10(): boolean {
    this.rustleGrass();
    const frame = this.sceneFrameCounter;
    if (frame === 0x20) {
      // ASM: depixel 22, 6 => y=22, x=6
      this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_WOOPER", 6 * 8, 22 * 8);
      this.audioEngine.playSound("SFX_INTRO_PICHU");
    }
    if (frame === 0x40) {
      // ASM: depixel 21, 16, 1, 0 => y=21, x=16
      this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_PICHU", 16 * 8, 21 * 8 + 1);
      this.audioEngine.playSound("SFX_INTRO_PICHU");
    }

    return frame === 0xc0;
  }

  private introScene11(): boolean {
    this.clearTilemap();
    this.clearSprites();
    // Visual parity note: this later Unown screen must start from a clean viewport.
    // Carrying over SCX/SCY from the grass scenes shifts the whole composition left.
    this.hSCX = 0;
    this.hSCY = 0;
    this.globalAnimXOffset = 0;
    this.drawTilemap("unowns", this.bgMap0);
    return true;
  }

  private introScene12(): boolean {
    const frame = this.sceneFrameCounter;
    for (const [soundFrame, sound] of IntroSequence.SCENE12_UNOWN_SOUNDS) {
      if (frame === soundFrame) {
        this.audioEngine.sfxChannelsOff();
        this.audioEngine.playSound(sound);
        break;
      }
    }

    if (frame >= 0xc0) {
      return true;
    }

    if (frame >= 0x80) {
      const c = frame;
      const timer = (c & 0xf) * 4;
      const a = (c & 0x70) | 0x40;
      const fadeParam = ((a >> 4) & 0x0f) | ((a << 4) & 0xf0);
      this.sceneTimer = timer;
      this.unownFade(fadeParam);
    } else {
      const c = frame;
      const timer = (c & 0x1f) * 2;
      const fadeParam = (c & 0xe0) >> 5;
      this.sceneTimer = timer;
      this.unownFade(fadeParam);
    }

    return false;
  }

  private introScene13(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("background", this.bgMap0);
    // ASM zeroes both hardware scroll registers after loading the forest BG.
    // Without this, stale vertical scroll makes valid tiles appear in the
    // wrong rows as Suicune runs through the scene.
    this.hSCX = 0;
    this.hSCY = 0;
    this.audioEngine.playMusic("MUSIC_CRYSTAL_OPENING", "intro");
    // ASM: engine/movie/intro.asm::IntroScene13 uses `depixel 13, 11, 4, 0`.
    // macros/gfx.asm::depixel is x tile, y tile, x pixel, y pixel.
    this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_SUICUNE", 13 * 8 + 4, 11 * 8);
    this.globalAnimXOffset = 0;
    return true;
  }

  private introScene14(): boolean {
    this.hSCX = (this.hSCX - 10) & 0xff;
    const frame = this.sceneFrameCounter;
    if (frame >= 0x80) {
      return true;
    } else if (frame >= 0x60) {
      if (frame === 0x60) {
        this.audioEngine.playSound("SFX_INTRO_SUICUNE_4");
      }
      this.sceneTimer = 1;
      if (this.globalAnimXOffset < 0x88) {
        this.clearSprites();
      } else {
        this.globalAnimXOffset = (this.globalAnimXOffset - 8) & 0xff;
      }
    } else if (frame >= 0x40) {
      this.globalAnimXOffset = (this.globalAnimXOffset - 2) & 0xff;
    }
    return false;
  }

  private introScene15(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("suicune_jump", this.bgMap0);
    this.hSCX = 0;
    this.hSCY = IntroSequence.GBC_SCREEN_HEIGHT_PX;
    // ASM: depixel 8, 5 => y=8, x=5
    this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_UNOWN_F", 5 * 8, 8 * 8);
    // ASM: depixel 12, 0 => y=12, x=0
    const suicuneSprite = this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY", 0, 12 * 8);
    suicuneSprite.gfx_name = "suicune_jump";
    return true;
  }

  private introScene16(): boolean {
    const frame = this.sceneFrameCounter;
    if (frame >= 0x80) {
      return true;
    }
    const phase = frame & 0x3;
    if (phase === 0) {
      this.coloredSuicuneFrameSwap();
    }
    if (this.hSCY !== 0) {
      this.hSCY = (this.hSCY + 8) & 0xff;
    }
    return false;
  }

  private introScene17(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("suicune_close", this.bgMap0);
    this.hSCX = 0;
    this.hSCY = 0;
    this.globalAnimXOffset = 0;
    return true;
  }

  private introScene18(): boolean {
    const frame = this.sceneFrameCounter;
    if (frame >= 0x60) {
      return true;
    }
    if (this.hSCX !== 0x60) {
      this.hSCX = (this.hSCX + 8) & 0xff;
    }
    return false;
  }

  private introScene19(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("suicune_back", this.bgMap0);
    // ASM: depixel 12, 0 => y=12, x=0
    this.spawnSprite("SPRITE_ANIM_OBJ_INTRO_SUICUNE_AWAY", 0, 12 * 8);
    this.hSCX = 0;
    this.hSCY = (-5 * IntroSequence.TILE_SIZE) & 0xff;
    this.globalAnimXOffset = 0;
    return true;
  }

  private introScene20(): boolean {
    const frame = this.sceneFrameCounter;
    if (frame >= 0x98) {
      return true;
    }
    if (frame < 0x28) {
      this.hSCY = (this.hSCY + 1) & 0xff;
    } else if (frame >= 0x40 && frame < 0x58) {
      const temp = (frame - 0x18) & 0xff;
      if ((temp & 0x3) === 0x3) {
        const timer = (temp & 0x1c) >> 2;
        this.sceneTimer = timer;
        this.appearUnown(0);
      }
    }
    this.coloredSuicuneFrameSwap();
    return false;
  }

  private introScene21(): boolean {
    this.coloredSuicuneFrameSwap();
    this.sceneFrameCounter = 0;
    this.sceneTimer = 0;
    return true;
  }

  private introScene22(): boolean {
    const frame = this.sceneFrameCounter;
    if (frame >= 8) {
      this.clearSprites();
      return true;
    }
    return false;
  }

  private introScene23(): boolean {
    return true;
  }

  private introScene24(): boolean {
    this.clearSurface(this.windowLayer);
    const frame = this.sceneFrameCounter;
    if (frame >= 0x20) {
      this.nextSceneFrameCounter = 0x40;
      return true;
    }
    if (frame & 0x3) {
      return false;
    }
    const fadeIndex = (frame & 0x1c) >> 2;
    this.applyScene24PaletteFade(fadeIndex);
    return false;
  }

  private introScene25(): boolean {
    const currentFrame = this.sceneFrameCounter;
    const nextFrame = (currentFrame - 1) & 0xff;

    this.nextSceneFrameCounter = nextFrame;
    return nextFrame === 0;
  }

  private introScene26(): boolean {
    this.clearTilemap();
    this.clearSprites();
    this.drawTilemap("crystal_unowns", this.bgMap0);
    return true;
  }

  private introScene27(): boolean {
    const frame = this.sceneFrameCounter;
    if (frame >= 0x80) {
      this.sceneFrameCounter = 0x80;
      this.nextSceneFrameCounter = 0x80;
      return true;
    }
    this.sceneTimer = frame & 0x0f;
    const c = frame;
    const fadeParam = (c & 0x70) >> 4;
    this.fadeUnownWordPals(fadeParam);
    return false;
  }

  private introScene28(): boolean {
    const currentFrame = this.sceneFrameCounter;
    if (currentFrame === 0) {
      return true;
    }
    if (currentFrame === 0x18) {
      this.clearBgPalettes();
    } else if (currentFrame === 0x08) {
      this.audioEngine.playSound("SFX_INTRO_WHOOSH");
    }

    const nextFrame = (currentFrame - 1) & 0xff;
    this.nextSceneFrameCounter = nextFrame;
    return false;
  }
}
