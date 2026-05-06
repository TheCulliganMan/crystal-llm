import { beginFlarePlotFrame, finishFlarePlotFrame } from "@pokecrystal/core/ui/flare-plot-renderer";
import fs from "fs";
import path from "path";
import { pathToFileURL } from "url";
import { AudioEngine } from "@pokecrystal/core/engine/systems/audio";
import { gbcWordToRgb } from "@pokecrystal/core/core/gbc-colors";
import { GameState } from "@pokecrystal/core/core/state";
import { SCREEN_TILE_WIDTH, TEXTBOX_HEIGHT_TILES, TEXTBOX_Y_TILES } from "@pokecrystal/core/core/text-constants";
import { PlayerGender } from "@pokecrystal/core/core/enums";
import { getAssetPath, getDataDir } from "@pokecrystal/core/core/paths";
import { find_audio_file, find_music_file } from "@pokecrystal/core/core/audio-formats";
import { isCancelEvent, isConfirmEvent, isKeyDownEvent, isStartEvent } from "@pokecrystal/core/input/controls";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";
import { BitmapFont } from "@pokecrystal/core/ui/text/bitmap-font";
import type { FontRenderer as TextboxFontRenderer } from "@pokecrystal/core/ui/textbox";
import {
  TILE_SIZE,
  fillScreen,
  SCREEN_HEIGHT,
} from "./rendering";
import { BootTextboxRenderer } from "./boot-textbox-renderer";
import { TimeSetScreen } from "./time-set-screen";
import {
  type TextSnapshotPayload,
} from "../../text-overlays";
import { buildOakIntroControlLines } from "../../control-lines";

// ASM reference: engine/menus/intro_menu.asm::OakSpeech

const ensureOakCanvasImageSource = (surface: InstanceType<typeof gameEngine.Surface>): CanvasImageSource => {
  return surface.canvas as CanvasImageSource;
};

const TIMESET_DOWN_ARROW_PATH = ["gfx", "new_game", "down_arrow.png"] as const;
const WHITE: [number, number, number] = [255, 255, 255];
const BLACK: [number, number, number] = [0, 0, 0];

export class OakIntroSequence {
  static readonly SPRITE_X = 48;
  static readonly SPRITE_Y = 32;
  static readonly FRONTPIC_SIZE = 56;

  private readonly timeSetScreen: TimeSetScreen;
  private timeSetComplete = false;

  private readonly sceneHandlers: Array<() => boolean>;
  private currentSprite: string | null = null;
  private currentSpriteSurface: InstanceType<typeof gameEngine.Surface> | null = null;
  private spriteType = "pokemon";
  private spriteFrame = 0;
  private animationTimer = 0;
  private animationSpeed = 15;

  private fadeFrameDelay = 8;
  private fadeActive = false;
  private fadeDirection: "in" | "out" = "in";
  private fadeTotalFrames = 1;
  private fadeElapsed = 0;
  private fadeAlpha = 0;
  private instantMode = false;

  private wipeActive = false;
  private wipeWindowX = 0;

  private downArrow: InstanceType<typeof gameEngine.Surface> | null = null;
  private readonly spriteCache = new Map<string, InstanceType<typeof gameEngine.Surface>>();
  private readonly spriteAnimationCache = new Map<string, InstanceType<typeof gameEngine.Surface>[]>();
  private readonly bitmapFont = new BitmapFont();
  private readonly bootTextboxRenderer: BootTextboxRenderer;

  private readonly textLibrary: Record<string, string[]> = {
    oak_text1: [
      "Hello! Sorry to\nkeep you waiting!",
      "Welcome to the\nworld of #MON!",
      "My name is OAK.",
      "People call me the\n#MON PROF.",
    ],
    oak_text2: [
      "This world is in-\nhabited by crea-\ntures that we call",
      "#MON.",
    ],
    oak_text4: [
      "People and #MON\nlive together by",
      "supporting each\nother.",
      "Some people play\nwith #MON, some\nbattle with them.",
    ],
    oak_text5: [
      "But we don't know\neverything about\n#MON yet.",
      "There are still\nmany mysteries to\nsolve.",
      "That's why I study\n#MON every day.",
    ],
    oak_text6: ["Now, what did you\nsay your name was?"],
    oak_text7: [
      "<PLAYER>, are you\nready?",
      "Your very own\n#MON story is\nabout to unfold.",
      "You'll face fun\ntimes and tough\nchallenges.",
      "A world of dreams\nand adventures",
      "with #MON\nawaits! Let's go!",
      "I'll be seeing you\nlater!",
    ],
  };

  private playerGender: PlayerGender = PlayerGender.MALE;
  private playerPicSurface: InstanceType<typeof gameEngine.Surface> | null = null;

  private mode: "intro" | "final" = "intro";
  private finished = false;
  private finalFinished = false;

  private sceneIndex = 0;
  private sceneState = "";
  private scenePhase = "init";
  private sceneFadeInSteps = 0;
  private sceneFadeOutSteps = 0;

  private blinkTimer = 0;

  private textQueue: string[] = [];
  private currentText = "";
  private visibleChars = 0;
  private textTimer = 0;
  private waitingForInput = false;

  private constructor(
    private readonly audioEngine: AudioEngine,
    private readonly gameState: GameState,
    font: TextboxFontRenderer
  ) {
    this.bootTextboxRenderer = new BootTextboxRenderer(font, TILE_SIZE);
    this.timeSetScreen = new TimeSetScreen(gameState, audioEngine, font);
    this.sceneHandlers = [
      this.sceneOakIntroFirst.bind(this),
      this.sceneWooperShowcase.bind(this),
      this.sceneOakIntroSecond.bind(this),
      this.scenePlayerPicture.bind(this),
    ];
    this.reset();
  }

  static async create(
    audioEngine: AudioEngine,
    gameState: GameState,
    font: TextboxFontRenderer
  ): Promise<OakIntroSequence> {
    const sequence = new OakIntroSequence(audioEngine, gameState, font);
    await sequence.loadAssets();
    return sequence;
  }

  private async loadAssets(): Promise<void> {
    await this.bitmapFont.load();
    this.downArrow = await this.loadSurface(TIMESET_DOWN_ARROW_PATH);
    await this.preloadSprite("oak", "trainer");
    await this.preloadSprite("wooper", "pokemon");
    await this.updatePlayerPicSurface();
  }

  reset(): void {
    this.resetSequenceState();
    this.timeSetScreen.reset();
    this.timeSetComplete = false;
  }

  setPlayerGender(gender: PlayerGender): void {
    if (gender === this.playerGender && this.playerPicSurface) {
      return;
    }
    this.playerGender = gender as any;
    void this.updatePlayerPicSurface();
  }

  setInstantMode(enabled: boolean): void {
    this.instantMode = enabled;
    if (enabled && this.fadeActive) {
      this.fadeActive = false;
      this.fadeAlpha = this.fadeDirection === "in" ? 0 : 255;
    }
  }

  private async updatePlayerPicSurface(): Promise<void> {
    const rawSurface = await this.loadSurface([
      "gfx",
      "player",
      this.playerGender === PlayerGender.MALE ? "chris.png" : "kris.png",
    ]);
    this.playerPicSurface = rawSurface ? this.applyPlayerPalette(rawSurface) : null;
  }

  private resetSequenceState(): void {
    this.mode = "intro";
    this.finished = false;
    this.finalFinished = false;

    this.sceneIndex = 0;
    this.sceneState = "";
    this.scenePhase = "init";
    this.sceneFadeInSteps = 0;
    this.sceneFadeOutSteps = 0;

    this.currentSprite = null;
    this.currentSpriteSurface = null;
    this.spriteFrame = 0;
    this.animationTimer = 0;
    this.blinkTimer = 0;

    this.fadeActive = false;
    this.fadeDirection = "in";
    this.fadeTotalFrames = 1;
    this.fadeElapsed = 0;
    this.fadeAlpha = 0;

    this.wipeActive = false;
    this.wipeWindowX = 0;

    this.textQueue = [];
    this.currentText = "";
    this.visibleChars = 0;
    this.textTimer = 0;
    this.waitingForInput = false;
  }

  update(): boolean {
    if (this.mode === "final") {
      return this.finalFinished;
    }

    if (!this.timeSetComplete) {
      this.timeSetScreen.update();
      if (!this.timeSetScreen.isFinished()) {
        return false;
      }
      this.timeSetComplete = true;
    }

    if (this.finished || this.sceneIndex >= this.sceneHandlers.length) {
      this.finished = true;
      return true;
    }

    this.blinkTimer = (this.blinkTimer + 1) % 60;
    this.updateFade();

    const sceneComplete = this.sceneHandlers[this.sceneIndex]();
    this.updateAnimation();

    if (sceneComplete) {
      this.sceneIndex += 1;
      this.sceneState = "";
      this.scenePhase = "init";
      this.currentSprite = null;
      this.currentSpriteSurface = null;
      this.spriteFrame = 0;
      this.animationTimer = 0;
      if (this.sceneIndex >= this.sceneHandlers.length) {
        this.finished = true;
      }
    }

    return this.finished;
  }

  handleInput(event: KeyboardEvent): boolean {
    if (!this.timeSetComplete) {
      this.timeSetScreen.handleInput(event);
      return false;
    }

    if (!isKeyDownEvent(event)) {
      return false;
    }

    if (isConfirmEvent(event) || isStartEvent(event)) {
      if (this.waitingForInput) {
        this.waitingForInput = false;
      } else if (this.currentText && this.visibleChars < this.currentText.length) {
        this.visibleChars = this.currentText.length;
        this.waitingForInput = true;
      }
      return false;
    }

    if (isCancelEvent(event) && this.mode === "intro") {
      this.finished = true;
      return true;
    }

    return false;
  }

  draw(ctx: CanvasRenderingContext2D): void {
    const flareStart = beginFlarePlotFrame();
    if (!this.timeSetComplete) {
      this.timeSetScreen.draw(ctx);
      const flareTarget =
        ctx.canvas && ((ctx.canvas as unknown as { __gameEngineSurface?: unknown }).__gameEngineSurface)
          ? (((ctx.canvas as unknown as { __gameEngineSurface: unknown }).__gameEngineSurface) as CanvasRenderingContext2D | InstanceType<typeof gameEngine.Surface>)
          : ctx;
      finishFlarePlotFrame(flareStart, "oak_intro", flareTarget, 0, 0, 100, 30);
      return;
    }

    fillScreen(ctx, [255, 255, 255]);

    if (this.currentSpriteSurface) {
      const spriteSource =
        this.currentSpriteSurface.getCanvasImageSource?.() ??
        (this.currentSpriteSurface.canvas as unknown as CanvasImageSource);
      ctx.drawImage(
        spriteSource,
        OakIntroSequence.SPRITE_X,
        OakIntroSequence.SPRITE_Y
      );
    } else if (this.currentSprite) {
      const cacheKey = this.spriteCacheKey(this.currentSprite, this.spriteType);
      const animationFrames = this.spriteAnimationCache.get(cacheKey);
      const spriteSurface =
        animationFrames && animationFrames.length > 0
          ? animationFrames[this.spriteFrame % animationFrames.length]
          : this.spriteCache.get(cacheKey);
      if (spriteSurface) {
        ctx.drawImage(
          ensureOakCanvasImageSource(spriteSurface),
          OakIntroSequence.SPRITE_X,
          OakIntroSequence.SPRITE_Y
        );
      }
    }

    if (this.currentText) {
      this.showTextBox(ctx, this.currentText.slice(0, this.visibleChars));
    }

    if (this.wipeActive) {
      this.drawWipeOverlay(ctx);
    }

    if (this.fadeActive || this.fadeAlpha > 0) {
      ctx.fillStyle = `rgba(255, 255, 255, ${this.fadeAlpha / 255})`;
      ctx.fillRect(0, 0, ctx.canvas.width, ctx.canvas.height);
    }
    finishFlarePlotFrame(flareStart, "oak_intro", ctx, 0, 0, 100, 30);
  }

  startFinalEncouragement(playerName: string): void {
    const name = playerName.trim().toUpperCase() || "PLAYER";
    const pages = this.textLibrary.oak_text7.map((page) => page.replace("<PLAYER>", name));
    this.mode = "final";
    this.finalFinished = false;
    this.currentSprite = null;
    this.currentSpriteSurface = null;
    this.spriteFrame = 0;
    this.animationTimer = 0;
    this.queueText(pages, true);
  }

  updateFinalEncouragement(): boolean {
    if (this.mode !== "final") {
      return false;
    }

    this.blinkTimer = (this.blinkTimer + 1) % 60;
    if (this.finalFinished) {
      return true;
    }
    if (this.advanceTextQueue()) {
      this.finalFinished = true;
    }
    return this.finalFinished;
  }

  handleFinalInput(event: KeyboardEvent): boolean {
    return this.handleInput(event);
  }

  getDebugState(): {
    mode: "intro" | "final";
    sceneIndex: number;
    sceneState: string;
    scenePhase: string;
    currentSprite: string | null;
    waitingForInput: boolean;
    timeSetComplete: boolean;
    timeSetPhase: string | null;
    visibleText: string;
  } {
    return {
      mode: this.mode,
      sceneIndex: this.sceneIndex,
      sceneState: this.sceneState,
      scenePhase: this.scenePhase,
      currentSprite: this.currentSprite,
      waitingForInput: this.waitingForInput,
      timeSetComplete: this.timeSetComplete,
      timeSetPhase: this.timeSetComplete ? null : this.timeSetScreen.getPhase(),
      visibleText: this.timeSetComplete ? this.currentText.slice(0, this.visibleChars) : "",
    };
  }

  getTextSnapshot(): TextSnapshotPayload {
    if (!this.timeSetComplete) {
      return this.timeSetScreen.getTextSnapshot();
    }

    const dialogueText = this.currentText.slice(0, this.visibleChars);
    const dialogueLines = dialogueText ? dialogueText.split("\n") : null;
    const spriteLabel = this.currentSprite
      ? this.currentSprite.toUpperCase()
      : this.currentSpriteSurface
        ? "PLAYER"
        : "NONE";
    const infoLines = [
      "STATE: oak_intro",
      `MODE: ${this.mode}`,
      `SCENE: ${this.sceneState || "none"}`,
      `PHASE: ${this.scenePhase}`,
      `WAITING: ${this.waitingForInput ? "yes" : "no"}`,
    ];
    infoLines.push(
      ...buildOakIntroControlLines({
        waitingForInput: this.waitingForInput,
        canRevealText: Boolean(this.currentText && this.visibleChars < this.currentText.length),
        allowSkip: this.mode === "intro",
      })
    );

    return {
      viewportLines: [
        this.mode === "final" ? "OAK FINALE" : "OAK INTRO",
        `SPRITE: ${spriteLabel}`,
      ],
      infoLines,
      viewportTitle: this.mode === "final" ? "Oak Finale" : "Oak Intro",
      infoTitle: this.mode === "final" ? "Oak Finale" : "Oak Intro",
      menuLines: null,
      promptLines: null,
      dialogueLines,
    };
  }

  // Scene handlers -----------------------------------------------------------
  private sceneOakIntroFirst(): boolean {
    if (this.sceneState !== "oak_intro_1") {
      this.ensureAudioAssets();
      this.audioEngine.playMusic("MUSIC_ROUTE_30", "intro");
      this.setupScene({
        name: "oak_intro_1",
        sprite: "oak",
        spriteType: "trainer",
        textPages: this.textLibrary.oak_text1,
        fadeInSteps: 4,
        fadeOutSteps: 3,
      });
    }
    return this.driveStandardScene();
  }

  private sceneWooperShowcase(): boolean {
    if (this.sceneState !== "wooper_showcase") {
      this.setupScene({
        name: "wooper_showcase",
        sprite: "wooper",
        spriteType: "pokemon",
        textPages: [],
        fadeInSteps: 0,
        fadeOutSteps: 3,
      });
      this.scenePhase = "wipe_in";
      this.startWipe();
    }
    if (this.scenePhase === "wipe_in") {
      if (this.advanceWipe()) {
        this.scenePhase = "text_one";
        this.queueText(this.textLibrary.oak_text2, true);
      }
    } else if (this.scenePhase === "text_one") {
      if (this.advanceTextQueue()) {
        this.scenePhase = "cry";
        this.audioEngine.playSound("wooper_cry");
      }
    } else if (this.scenePhase === "cry") {
      if (!this.audioEngine.isSoundPlaying("wooper_cry")) {
        this.scenePhase = "text_two";
        this.queueText(this.textLibrary.oak_text4, true);
      }
    } else if (this.scenePhase === "text_two") {
      if (this.advanceTextQueue()) {
        this.scenePhase = "fade_out";
        this.startFade("out", this.sceneFadeOutSteps);
      }
    } else if (this.scenePhase === "fade_out") {
      if (!this.fadeActive) {
        this.currentSprite = null;
        this.scenePhase = "complete";
      }
    }
    return this.scenePhase === "complete";
  }

  private sceneOakIntroSecond(): boolean {
    if (this.sceneState !== "oak_intro_2") {
      this.setupScene({
        name: "oak_intro_2",
        sprite: "oak",
        spriteType: "trainer",
        textPages: this.textLibrary.oak_text5,
        fadeInSteps: 3,
        fadeOutSteps: 3,
      });
    }
    return this.driveStandardScene();
  }

  private scenePlayerPicture(): boolean {
    if (this.sceneState !== "player_picture") {
      this.setupScene({
        name: "player_picture",
        sprite: null,
        spriteType: "trainer",
        textPages: this.textLibrary.oak_text6,
        fadeInSteps: 3,
        fadeOutSteps: 0,
        spriteSurface: this.playerPicSurface,
      });
    }
    const completed = this.driveStandardScene();
    if (completed) {
      this.scenePhase = "complete";
    }
    return this.scenePhase === "complete";
  }

  private setupScene({
    name,
    sprite,
    spriteType,
    textPages,
    fadeInSteps,
    fadeOutSteps,
    spriteSurface,
  }: {
    name: string;
    sprite: string | null;
    spriteType: string;
    textPages: string[];
    fadeInSteps: number;
    fadeOutSteps: number;
    spriteSurface?: InstanceType<typeof gameEngine.Surface> | null;
  }): void {
    this.sceneState = name;
    this.scenePhase = fadeInSteps && !this.instantMode ? "fade_in" : "text";
    this.sceneFadeInSteps = fadeInSteps;
    this.sceneFadeOutSteps = fadeOutSteps;
    this.currentSprite = sprite;
    this.currentSpriteSurface = spriteSurface ?? null;
    this.spriteType = spriteType;
    this.spriteFrame = 0;
    this.animationTimer = 0;
    this.queueText(textPages, true);
    if (fadeInSteps && !this.instantMode) {
      this.startFade("in", fadeInSteps);
    } else {
      this.fadeActive = false;
      this.fadeAlpha = 0;
    }
  }

  private driveStandardScene(): boolean {
    if (this.scenePhase === "fade_in") {
      if (!this.fadeActive && this.fadeAlpha === 0) {
        this.scenePhase = "text";
      }
    }
    if (this.scenePhase === "text") {
      if (this.advanceTextQueue()) {
        if (this.sceneFadeOutSteps) {
          this.scenePhase = "fade_out";
          this.startFade("out", this.sceneFadeOutSteps);
        } else {
          this.scenePhase = "complete";
        }
      }
    }
    if (this.scenePhase === "fade_out") {
      if (!this.fadeActive) {
        this.currentSprite = null;
        this.currentSpriteSurface = null;
        this.scenePhase = "complete";
      }
    }
    return this.scenePhase === "complete";
  }

  private queueText(pages: string[], reset = false): void {
    if (reset) {
      this.textQueue = [...pages];
      this.currentText = "";
      this.visibleChars = 0;
      this.textTimer = 0;
      this.waitingForInput = false;
    } else {
      this.textQueue.push(...pages);
    }
  }

  private advanceTextQueue(): boolean {
    if (!this.currentText && this.textQueue.length === 0) {
      return true;
    }

    if (!this.currentText) {
      this.currentText = this.textQueue.shift() ?? "";
      this.visibleChars = this.instantMode ? this.currentText.length : 0;
      this.textTimer = 0;
      this.waitingForInput = this.instantMode && this.currentText.length > 0;
      return false;
    }

    if (this.visibleChars < this.currentText.length) {
      if (this.instantMode) {
        this.visibleChars = this.currentText.length;
        this.waitingForInput = true;
        return false;
      }
      this.textTimer += 1;
      if (this.textTimer >= 2) {
        this.textTimer = 0;
        this.visibleChars += 1;
        if (this.visibleChars >= this.currentText.length) {
          this.waitingForInput = true;
        }
      }
      return false;
    }

    if (this.waitingForInput) {
      return false;
    }

    this.currentText = "";
    return false;
  }

  private showTextBox(ctx: CanvasRenderingContext2D, text: string): void {
    this.bootTextboxRenderer.drawTextBox(
      ctx,
      text,
      0,
      TEXTBOX_Y_TILES,
      SCREEN_TILE_WIDTH,
      TEXTBOX_HEIGHT_TILES
    );

    if (this.waitingForInput) {
      this.drawPromptArrow(ctx);
    }
  }

  private drawPromptArrow(ctx: CanvasRenderingContext2D): void {
    if (this.downArrow && this.blinkTimer < 30) {
      const arrowX = 18 * TILE_SIZE;
      const arrowY = 16 * TILE_SIZE;
      this.bootTextboxRenderer.drawPromptArrow(ctx, this.downArrow, arrowX, arrowY);
    }
  }

  private startFade(direction: "in" | "out", steps: number): void {
    this.fadeDirection = direction;
    if (this.instantMode) {
      this.fadeTotalFrames = 1;
      this.fadeElapsed = 1;
      this.fadeActive = false;
      this.fadeAlpha = direction === "in" ? 0 : 255;
      return;
    }
    this.fadeTotalFrames = Math.max(1, steps * this.fadeFrameDelay);
    this.fadeElapsed = 0;
    this.fadeActive = true;
    this.fadeAlpha = direction === "in" ? 255 : 0;
  }

  private updateFade(): void {
    if (!this.fadeActive) {
      return;
    }
    this.fadeElapsed += 1;
    const progress = Math.min(1, this.fadeElapsed / this.fadeTotalFrames);
    let rawAlpha = 0;
    if (this.fadeDirection === "in") {
      rawAlpha = Math.max(0, Math.floor(255 * (1 - progress)));
    } else {
      rawAlpha = Math.min(255, Math.floor(255 * progress));
    }

    const step = Math.floor(rawAlpha / 8);
    this.fadeAlpha = Math.floor((step * 255) / 31);

    if (this.fadeElapsed >= this.fadeTotalFrames) {
      this.fadeActive = false;
      this.fadeAlpha = this.fadeDirection === "in" ? 0 : 255;
    }
  }

  private startWipe(): void {
    if (this.instantMode) {
      this.wipeActive = false;
      this.wipeWindowX = 168;
      return;
    }
    this.wipeActive = true;
    this.wipeWindowX = 0;
  }

  private advanceWipe(): boolean {
    if (!this.wipeActive) {
      return true;
    }
    this.wipeWindowX += 8;
    if (this.wipeWindowX > 160) {
      this.wipeActive = false;
      return true;
    }
    return false;
  }

  private drawWipeOverlay(ctx: CanvasRenderingContext2D): void {
    const rectX = this.wipeWindowX;
    const width = 160 - rectX;
    if (width <= 0) {
      return;
    }
    ctx.fillStyle = "#fff";
    ctx.fillRect(rectX, 0, width, SCREEN_HEIGHT);
  }

  private updateAnimation(): void {
    if (!this.currentSprite || this.currentSprite !== "wooper") {
      this.spriteFrame = 0;
      this.animationTimer = 0;
      return;
    }
    const cacheKey = this.spriteCacheKey(this.currentSprite, this.spriteType);
    const frameCount = this.spriteAnimationCache.get(cacheKey)?.length ?? 0;
    if (frameCount <= 1) {
      this.spriteFrame = 0;
      this.animationTimer = 0;
      return;
    }
    this.animationTimer += 1;
    if (this.animationTimer >= this.animationSpeed) {
      this.animationTimer = 0;
      this.spriteFrame = (this.spriteFrame + 1) % frameCount;
    }
  }

  private ensureAudioAssets(): void {
    const audioRoot = path.join(getDataDir(), "audio");
    const route30Path = find_music_file(audioRoot, "route30");
    const wooperCry = find_audio_file(path.join(audioRoot, "cries"), "wooper");
    if (route30Path && !this.audioEngine.music.route30) {
      this.audioEngine.loadMusic("route30", route30Path);
    }
    if (wooperCry && !this.audioEngine.sounds.wooper_cry) {
      this.audioEngine.loadSound("wooper_cry", wooperCry);
    }
  }

  private async loadSurface(parts: readonly string[]): Promise<InstanceType<typeof gameEngine.Surface> | null> {
    const assetPath = getAssetPath(...parts);
    const url = typeof window === "undefined"
      ? pathToFileURL(assetPath).toString()
      : assetPath;
    try {
      const surface = await gameEngine.image.load(url);
      return surface;
    } catch {
      return null;
    }
  }

  private spriteCacheKey(name: string, spriteType: string): string {
    return `${spriteType}:${name}`;
  }

  private async preloadSprite(name: string, spriteType: "trainer" | "pokemon"): Promise<void> {
    const key = this.spriteCacheKey(name, spriteType);
    if (this.spriteCache.has(key)) {
      return;
    }
    let parts: string[];
    if (spriteType === "trainer") {
      parts = ["gfx", "trainers", `${name}.png`];
    } else {
      parts = ["gfx", "pokemon", name, "front.png"];
    }
    const surface = await this.loadSurface(parts);
    if (surface) {
      if (spriteType === "pokemon") {
        const frames = this.extractPokemonFrames(surface).map((frame) => this.normalizeFrontpicSurface(frame));
        this.spriteAnimationCache.set(key, frames);
        if (frames[0]) {
          this.spriteCache.set(key, frames[0]);
          return;
        }
      }
      this.spriteCache.set(key, this.normalizeFrontpicSurface(surface));
    }
  }

  private extractPokemonFrames(
    source: InstanceType<typeof gameEngine.Surface>
  ): InstanceType<typeof gameEngine.Surface>[] {
    const [width, height] = source.get_size();
    if (height === width) {
      return [source.copy()];
    }
    if (height <= width || height % width !== 0) {
      throw new Error("Oak intro animated frontpic must be a square-frame strip.");
    }
    const frameCount = height / width;
    const frames: InstanceType<typeof gameEngine.Surface>[] = [];
    for (let i = 0; i < frameCount; i += 1) {
      const rect = new gameEngine.Rect(0, i * width, width, width);
      frames.push(source.subsurface(rect).copy());
    }
    return frames;
  }

  private normalizeFrontpicSurface(
    source: InstanceType<typeof gameEngine.Surface>
  ): InstanceType<typeof gameEngine.Surface> {
    const [width, height] = source.get_size();
    if (width === OakIntroSequence.FRONTPIC_SIZE && height === OakIntroSequence.FRONTPIC_SIZE) {
      return source.copy();
    }
    const normalized = new gameEngine.Surface(OakIntroSequence.FRONTPIC_SIZE, OakIntroSequence.FRONTPIC_SIZE);
    normalized.fill([255, 255, 255, 255]);
    const offsetX = Math.max(0, Math.floor((OakIntroSequence.FRONTPIC_SIZE - width) / 2));
    const offsetY = Math.max(0, Math.floor((OakIntroSequence.FRONTPIC_SIZE - height) / 2));
    normalized.blit(source, [offsetX, offsetY]);
    return normalized;
  }

  private applyPlayerPalette(
    source: InstanceType<typeof gameEngine.Surface>
  ): InstanceType<typeof gameEngine.Surface> {
    const palette = this.loadPlayerPalette();
    const recolored = source.copy();
    const [width, height] = recolored.get_size();
    for (let y = 0; y < height; y += 1) {
      for (let x = 0; x < width; x += 1) {
        const [r, g, b, a] = recolored.get_at([x, y]);
        if (a === 0) {
          continue;
        }
        const grayscale = Math.round((r + g + b) / 3);
        const paletteIndex = this.playerPaletteIndexFromGray(grayscale);
        const [pr, pg, pb] = palette[paletteIndex] ?? palette[0];
        recolored.set_at([x, y], [pr, pg, pb, a]);
      }
    }
    return recolored;
  }

  private playerPaletteIndexFromGray(gray: number): number {
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
  }

  private loadPlayerPalette(): [number, number, number][] {
    const paletteStem = this.playerGender === PlayerGender.FEMALE ? "falkner" : "cal";
    const palettePath = getAssetPath("gfx", "trainers", `${paletteStem}.gbcpal`);
    if (!fs.existsSync(palettePath)) {
      throw new Error(`Missing player intro palette: ${palettePath}`);
    }
    const data = fs.readFileSync(palettePath);
    if (data.length < 8) {
      throw new Error(`Player intro palette ${palettePath} must be at least 8 bytes, got ${data.length}.`);
    }
    const colours: [number, number, number][] = [];
    for (let offset = 0; offset < 8; offset += 2) {
      colours.push(gbcWordToRgb(data.readUInt16LE(offset)));
    }
    if (colours.length < 3) {
      throw new Error(`Player intro palette ${palettePath} must define at least three colours.`);
    }
    // ASM parity: engine/gfx/color.asm::GetPlayerOrMonPalettePointer uses PlayerPalette/KrisPalette middle colours.
    return [WHITE, colours[1], colours[2], BLACK];
  }
}
