import fs from "fs";
import { gbc5To8 } from "@pokecrystal/core/core/gbc-colors";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";
import { getAssetPath } from "@pokecrystal/core/core/paths";
import { gameEngine, Surface, Rect } from "@pokecrystal/core/ui/game-engine";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile";
import { ScreenUI } from "@pokecrystal/core/ui/screens/screen-types";

type HealAnimationEvent =
  | { frame: number; kind: "load_gfx"; payload: string }
  | { frame: number; kind: "static_sprite"; payload: string }
  | { frame: number; kind: "music"; payload: string }
  | { frame: number; kind: "sfx"; payload: string }
  | { frame: number; kind: "wait_sfx"; payload: { waitFor: string; thenPlay?: string } }
  | { frame: number; kind: "spawn_ball"; payload: number }
  | { frame: number; kind: "flash_toggle"; payload: number };

class HealAnimationTimeline {
  constructor(
    public readonly spawnFrames: number[],
    public readonly flashFrames: number[],
    public readonly flashStart: number,
    public readonly flashDelay: number,
    public readonly flashIterations: number,
    public readonly events: HealAnimationEvent[],
    public readonly totalFrames: number
  ) {}

  get flashDuration(): number {
    return this.flashIterations ? this.flashDelay * this.flashIterations : 0;
  }

  get musicStartFrame(): number {
    for (const event of this.events) {
      if (event.kind === "music") {
        return event.frame;
      }
    }
    return this.flashStart;
  }
}

type OverworldLike = {
  ui?: ScreenUI;
  audio_engine?: {
    playMusic?: (name: string, role?: string) => void;
    playSound?: (name: string) => void;
    isSoundPlaying?: (name: string) => boolean;
  };
  update?: () => void;
  draw?: () => void;
  handleInput?: (event: unknown) => void;
};

const FRAME_DURATION_MS = GB_FRAME_DURATION_MS;
const MAX_PARTY_SLOTS = 6;
const HEAL_MACHINE_TILE_BASE = 0x7c;

export class HealMachineAnimator {
  // ASM mapping: pokecrystal_disassembly/engine/events/heal_machine_anim.asm
  private static readonly HEAL_MACHINE_IMAGE_PATH = getAssetPath(
    "gfx",
    "overworld",
    "heal_machine.png"
  );
  private static readonly HEAL_MACHINE_2BPP_PATH = getAssetPath(
    "gfx",
    "overworld",
    "heal_machine.2bpp"
  );
  private static readonly HEAL_MACHINE_PALETTE_PATH = getAssetPath(
    "gfx",
    "overworld",
    "heal_machine.pal"
  );
  private static readonly HEAL_MACHINE_PALETTE_FALLBACK_5BIT: Array<[number, number, number]> = [
    [31, 31, 31],
    [31, 19, 10],
    [31, 7, 1],
    [0, 0, 0],
  ];
  private static readonly HEAL_MACHINE_INDEXED_GRAY_PALETTE: Array<[number, number, number]> = [
    [255, 255, 255],
    [170, 170, 170],
    [85, 85, 85],
    [0, 0, 0],
  ];
  private static healMachineSurfaces: Map<string, Surface | null> = new Map();
  private static healMachineTiles: Surface[] | null = null;
  private static healMachineTilesByPalette: Map<number, Surface[]> = new Map();
  private static healMachinePalette: Array<[number, number, number]> | null = null;
  private static pokeballSurfaces: Map<string, Surface> = new Map();

  static readonly HEAL_MACHINE_FLASH_DELAY = 10;
  static readonly HEAL_MACHINE_FLASH_ITERATIONS = 8;
  static readonly HEAL_MACHINE_BALL_INTERVAL = 30;
  static readonly OAM_X_BIAS = 8;
  static readonly OAM_Y_BIAS = 16;
  // ASM: .PlaceHealingMachineTile applies bcpixel 2,4 only for HEALMACHINE_ELMS_LAB.
  static readonly HEAL_MACHINE_PLACEMENT_OFFSETS: Record<number, [number, number]> = {
    0: [0, 0],
    1: [2 * TILE_SIZE, 4 * TILE_SIZE],
    2: [0, 0],
  };
  static readonly HEAL_MACHINE_STATIC_SPRITES: Record<number, number> = {
    0: 2,
    1: 2,
    2: 0,
  };
  static readonly HEAL_MACHINE_TYPE_MAP: Record<string, number> = {
    HEALMACHINE_POKECENTER: 0,
    HEALMACHINE_ELMS_LAB: 1,
    HEALMACHINE_HALL_OF_FAME: 2,
  };
  static readonly HEAL_MACHINE_OAM_LAYOUTS: Record<
    number,
    Array<[number, number, number, number, number, number]>
  > = {
    0: [
      [4, 4, 2, 0, 0x7c, 0],
      [4, 4, 6, 0, 0x7c, 0],
      [4, 4, 0, 6, 0x7d, 0],
      [5, 4, 0, 6, 0x7d, 0x20],
      [4, 5, 0, 3, 0x7d, 0],
      [5, 5, 0, 3, 0x7d, 0x20],
      [4, 6, 0, 0, 0x7d, 0],
      [5, 6, 0, 0, 0x7d, 0x20],
    ],
    1: [
      [4, 4, 2, 0, 0x7c, 0],
      [4, 4, 6, 0, 0x7c, 0],
      [4, 4, 0, 6, 0x7d, 0],
      [5, 4, 0, 6, 0x7d, 0x20],
      [4, 5, 0, 3, 0x7d, 0],
      [5, 5, 0, 3, 0x7d, 0x20],
      [4, 6, 0, 0, 0x7d, 0],
      [5, 6, 0, 0, 0x7d, 0x20],
    ],
    2: [
      [10, 7, 1, 4, 0x7d, 0],
      [10, 7, 6, 4, 0x7d, 0],
      [9, 7, 5, 3, 0x7d, 0],
      [11, 7, 2, 3, 0x7d, 0],
      [9, 7, 1, 1, 0x7d, 0],
      [11, 7, 5, 1, 0x7d, 0],
    ],
  };

  play(
    animationId: string | null,
    partySlots: number,
    overworld: OverworldLike | null
  ): void {
    // ASM: engine/events/heal_machine_anim.asm::HealMachineAnim
    const animationType = this.resolveHealMachineType(animationId);
    const slots = this.boundedPartySlots(partySlots, animationType);
    if (slots <= 0) {
      return;
    }
    const timeline = this.computeHealMachineTimeline(slots, animationType);
    const audioEngine = overworld?.audio_engine ?? null;
    this.runHealMachineAnimation(
      overworld,
      animationType,
      timeline,
      slots,
      audioEngine
    );
  }

  async playAsync(
    animationId: string | null,
    partySlots: number,
    overworld: OverworldLike | null
  ): Promise<void> {
    // Keep node/headless parity with the blocking path used by existing tests.
    if (typeof requestAnimationFrame !== "function") {
      this.play(animationId, partySlots, overworld);
      return;
    }
    const animationType = this.resolveHealMachineType(animationId);
    const slots = this.boundedPartySlots(partySlots, animationType);
    if (slots <= 0) {
      return;
    }
    const timeline = this.computeHealMachineTimeline(slots, animationType);
    const audioEngine = overworld?.audio_engine ?? null;
    await this.runHealMachineAnimationAsync(
      overworld,
      animationType,
      timeline,
      slots,
      audioEngine
    );
  }

  private runHealMachineAnimation(
    overworld: OverworldLike | null,
    animationType: number,
    timeline: HealAnimationTimeline,
    partySlots: number,
    audioEngine: OverworldLike["audio_engine"] | null
  ): void {
    if (timeline.totalFrames <= 0) {
      return;
    }
    const ui = overworld?.ui;
    const screen = ui?.screen ?? null;
    const hasScreen = Boolean(ui && screen);
    const anchor = hasScreen ? this.resolveOamAnchor(animationType) : [0, 0];
    const machineBounds = hasScreen && screen
      ? this.createHealMachineBounds(screen, animationType, anchor as [number, number])
      : null;
    const clampDx = machineBounds ? machineBounds.x - anchor[0] : 0;
    const clampDy = machineBounds ? machineBounds.y - anchor[1] : 0;
    const machineSpritesByPalette = new Map<number, Array<{ x: number; y: number; surface: Surface }>>();
    const scaledMachineSurfaces = new Map<number, Surface | null>();

    const renderFrame = (frame: number): void => {
      if (!hasScreen || !screen || !machineBounds) {
        gameEngine.time.delay(FRAME_DURATION_MS);
        return;
      }
      const visible = this.determineVisibleBallCount(frame, timeline, partySlots, animationType);
      const paletteIndex = this.paletteRotationIndex(frame, timeline);
      const staticCount = HealMachineAnimator.HEAL_MACHINE_STATIC_SPRITES[animationType] ?? 0;
      const scaledMachineSurface = this.scaledMachineSurfaceForPalette(
        animationType,
        paletteIndex,
        machineBounds,
        scaledMachineSurfaces
      );
      const machineSprites =
        machineSpritesByPalette.get(paletteIndex) ??
        this.computeHealMachineSprites(animationType, clampDx, clampDy, paletteIndex);
      machineSpritesByPalette.set(paletteIndex, machineSprites);

      // Draw the overworld first so the heal machine blit remains visible on top.
      overworld?.update?.();
      overworld?.draw?.();

      if (scaledMachineSurface) {
        screen.blit(scaledMachineSurface, [machineBounds.x, machineBounds.y]);
      }
      for (let i = 0; i < Math.min(staticCount, machineSprites.length); i += 1) {
        const sprite = machineSprites[i];
        screen.blit(sprite.surface, [sprite.x, sprite.y]);
      }
      for (
        let i = staticCount;
        i < Math.min(staticCount + visible, machineSprites.length);
        i += 1
      ) {
        const sprite = machineSprites[i];
        screen.blit(sprite.surface, [sprite.x, sprite.y]);
      }
      ui?.update?.();
      gameEngine.time.delay(FRAME_DURATION_MS);
    };

    const frameEvents = this.groupEventsByFrame(timeline.events);
    let frame = 0;
    while (frame < timeline.totalFrames) {
      const events = frameEvents.get(frame) ?? [];
      const waitIndex = events.findIndex((event) => event.kind === "wait_sfx");
      const initialEvents = waitIndex >= 0 ? events.slice(0, waitIndex) : events;
      this.dispatchAudioEvents(initialEvents, audioEngine);
      renderFrame(frame);
      frame += 1;
      if (waitIndex >= 0) {
        const waitEvent = events[waitIndex] as Extract<HealAnimationEvent, { kind: "wait_sfx" }>;
        const extraFrames = this.waitForSfxToFinish(
          audioEngine ?? null,
          waitEvent.payload.waitFor,
          frame,
          renderFrame
        );
        frame += extraFrames;
        const followUpEvents = events
          .slice(waitIndex + 1)
          .filter((event) => event.kind !== "wait_sfx");
        if (waitEvent.payload.thenPlay) {
          followUpEvents.unshift({
            frame,
            kind: "sfx",
            payload: waitEvent.payload.thenPlay,
          });
        }
        this.dispatchAudioEvents(followUpEvents, audioEngine);
      }
    }
  }

  private async runHealMachineAnimationAsync(
    overworld: OverworldLike | null,
    animationType: number,
    timeline: HealAnimationTimeline,
    partySlots: number,
    audioEngine: OverworldLike["audio_engine"] | null
  ): Promise<void> {
    if (timeline.totalFrames <= 0) {
      return;
    }
    const ui = overworld?.ui;
    const screen = ui?.screen ?? null;
    const hasScreen = Boolean(ui && screen);
    const anchor = hasScreen ? this.resolveOamAnchor(animationType) : [0, 0];
    const machineBounds = hasScreen && screen
      ? this.createHealMachineBounds(screen, animationType, anchor as [number, number])
      : null;
    const clampDx = machineBounds ? machineBounds.x - anchor[0] : 0;
    const clampDy = machineBounds ? machineBounds.y - anchor[1] : 0;
    const machineSpritesByPalette = new Map<number, Array<{ x: number; y: number; surface: Surface }>>();
    const scaledMachineSurfaces = new Map<number, Surface | null>();

    const renderFrame = async (frame: number): Promise<void> => {
      if (!hasScreen || !screen || !machineBounds) {
        await this.awaitHealMachineFrameAsync();
        return;
      }
      const visible = this.determineVisibleBallCount(frame, timeline, partySlots, animationType);
      const paletteIndex = this.paletteRotationIndex(frame, timeline);
      const staticCount = HealMachineAnimator.HEAL_MACHINE_STATIC_SPRITES[animationType] ?? 0;
      const scaledMachineSurface = this.scaledMachineSurfaceForPalette(
        animationType,
        paletteIndex,
        machineBounds,
        scaledMachineSurfaces
      );
      const machineSprites =
        machineSpritesByPalette.get(paletteIndex) ??
        this.computeHealMachineSprites(animationType, clampDx, clampDy, paletteIndex);
      machineSpritesByPalette.set(paletteIndex, machineSprites);

      // Draw the overworld first so the heal machine blit remains visible on top.
      overworld?.update?.();
      overworld?.draw?.();

      if (scaledMachineSurface) {
        screen.blit(scaledMachineSurface, [machineBounds.x, machineBounds.y]);
      }
      for (let i = 0; i < Math.min(staticCount, machineSprites.length); i += 1) {
        const sprite = machineSprites[i];
        screen.blit(sprite.surface, [sprite.x, sprite.y]);
      }
      for (
        let i = staticCount;
        i < Math.min(staticCount + visible, machineSprites.length);
        i += 1
      ) {
        const sprite = machineSprites[i];
        screen.blit(sprite.surface, [sprite.x, sprite.y]);
      }
      ui?.update?.();
      await this.awaitHealMachineFrameAsync();
    };

    const frameEvents = this.groupEventsByFrame(timeline.events);
    let frame = 0;
    while (frame < timeline.totalFrames) {
      const events = frameEvents.get(frame) ?? [];
      const waitIndex = events.findIndex((event) => event.kind === "wait_sfx");
      const initialEvents = waitIndex >= 0 ? events.slice(0, waitIndex) : events;
      this.dispatchAudioEvents(initialEvents, audioEngine);
      await renderFrame(frame);
      frame += 1;
      if (waitIndex >= 0) {
        const waitEvent = events[waitIndex] as Extract<HealAnimationEvent, { kind: "wait_sfx" }>;
        const extraFrames = await this.waitForSfxToFinishAsync(
          audioEngine ?? null,
          waitEvent.payload.waitFor,
          frame,
          renderFrame
        );
        frame += extraFrames;
        const followUpEvents = events
          .slice(waitIndex + 1)
          .filter((event) => event.kind !== "wait_sfx");
        if (waitEvent.payload.thenPlay) {
          followUpEvents.unshift({
            frame,
            kind: "sfx",
            payload: waitEvent.payload.thenPlay,
          });
        }
        this.dispatchAudioEvents(followUpEvents, audioEngine);
      }
    }
  }

  private computeHealMachineTimeline(
    partySlots: number,
    animationType: number
  ): HealAnimationTimeline {
    const slots = this.boundedPartySlots(partySlots, animationType);
    if (slots <= 0) {
      return new HealAnimationTimeline([], [], 0, 0, 0, [], 0);
    }

    const events: HealAnimationEvent[] = [{ frame: 0, kind: "load_gfx", payload: "heal_machine" }];
    let frame = 0;
    const staticCount = HealMachineAnimator.HEAL_MACHINE_STATIC_SPRITES[animationType] ?? 0;
    for (let i = 0; i < staticCount; i += 1) {
      events.push({ frame, kind: "static_sprite", payload: `static_${i}` });
    }

    if (animationType === HealMachineAnimator.HEAL_MACHINE_TYPE_MAP.HEALMACHINE_HALL_OF_FAME) {
      frame = this.simulateHallOfFameSequence(events, frame, slots);
    } else {
      frame = this.simulatePokeCenterSequence(events, frame, slots);
    }

    const spawnFrames = events.filter((event) => event.kind === "spawn_ball").map((event) => event.frame);
    const flashFrames = events.filter((event) => event.kind === "flash_toggle").map((event) => event.frame);
    const flashStart = flashFrames.length ? flashFrames[0] : 0;
    const flashDelay = flashFrames.length ? HealMachineAnimator.HEAL_MACHINE_FLASH_DELAY : 0;
    const flashIterations = flashFrames.length;
    const lastEventFrame = events.length ? events[events.length - 1].frame : 0;
    const totalFrames = Math.max(frame, lastEventFrame + 1);

    return new HealAnimationTimeline(
      spawnFrames,
      flashFrames,
      flashStart,
      flashDelay,
      flashIterations,
      events,
      totalFrames
    );
  }

  private simulatePokeCenterSequence(
    events: HealAnimationEvent[],
    frame: number,
    slots: number
  ): number {
    frame = this.appendPartyBallEvents(events, frame, slots);
    events.push({ frame, kind: "music", payload: "MUSIC_HEAL" });
    return this.appendFlashSequence(events, frame);
  }

  private simulateHallOfFameSequence(
    events: HealAnimationEvent[],
    frame: number,
    slots: number
  ): number {
    frame = this.appendPartyBallEvents(events, frame, slots);
    events.push({ frame, kind: "sfx", payload: "SFX_GAME_FREAK_LOGO_GS" });
    frame = this.appendFlashSequence(events, frame);
    // ASM: engine/events/heal_machine_anim.asm::HOF_PlaySFX WaitSFX before SFX_BOOT_PC.
    events.push({
      frame,
      kind: "wait_sfx",
      payload: { waitFor: "SFX_GAME_FREAK_LOGO_GS", thenPlay: "SFX_BOOT_PC" },
    });
    return frame;
  }

  private appendPartyBallEvents(
    events: HealAnimationEvent[],
    frame: number,
    slots: number
  ): number {
    const interval = Math.max(1, HealMachineAnimator.HEAL_MACHINE_BALL_INTERVAL);
    for (let slot = 0; slot < slots; slot += 1) {
      events.push({ frame, kind: "spawn_ball", payload: slot });
      events.push({ frame, kind: "sfx", payload: "SFX_SECOND_PART_OF_ITEMFINDER" });
      frame += interval;
    }
    return frame;
  }

  private appendFlashSequence(events: HealAnimationEvent[], frame: number): number {
    const delay = Math.max(1, HealMachineAnimator.HEAL_MACHINE_FLASH_DELAY);
    const iterations = Math.max(1, HealMachineAnimator.HEAL_MACHINE_FLASH_ITERATIONS);
    for (let index = 0; index < iterations; index += 1) {
      events.push({ frame, kind: "flash_toggle", payload: index });
      frame += delay;
    }
    return frame;
  }

  private groupEventsByFrame(events: HealAnimationEvent[]): Map<number, HealAnimationEvent[]> {
    const grouped = new Map<number, HealAnimationEvent[]>();
    for (const event of events) {
      const bucket = grouped.get(event.frame) ?? [];
      bucket.push(event);
      grouped.set(event.frame, bucket);
    }
    return grouped;
  }

  private dispatchAudioEvents(
    events: HealAnimationEvent[],
    audioEngine: OverworldLike["audio_engine"] | null | undefined
  ): void {
    if (!audioEngine) {
      return;
    }
    for (const event of events) {
      if (event.kind === "music" && typeof audioEngine.playSound === "function") {
        // ASM: PlayMusic MUSIC_HEAL plays the jingle once; the GB music track
        // ends naturally.  Using playSound (non-looping) instead of playMusic
        // (which sets loop=true) so the heal jingle doesn't repeat forever.
        audioEngine.playSound(this.resolveMusicIdentifier(event.payload));
      }
      if (event.kind === "sfx" && typeof audioEngine.playSound === "function") {
        audioEngine.playSound(event.payload);
      }
    }
  }

  private waitForSfxToFinish(
    audioEngine: OverworldLike["audio_engine"] | null,
    sfxName: string,
    startFrame: number,
    renderFrame: (frame: number) => void
  ): number {
    if (!audioEngine || typeof audioEngine.isSoundPlaying !== "function") {
      return 0;
    }
    const shouldTimeout = typeof process !== "undefined" && process.env?.NODE_ENV === "test";
    const maxFrames = shouldTimeout ? 600 : Number.POSITIVE_INFINITY;
    let waited = 0;
    while (audioEngine.isSoundPlaying(sfxName)) {
      if (waited >= maxFrames) {
        throw new Error(`Heal machine wait_sfx timed out waiting for ${sfxName}.`);
      }
      renderFrame(startFrame + waited);
      waited += 1;
    }
    return waited;
  }

  private async waitForSfxToFinishAsync(
    audioEngine: OverworldLike["audio_engine"] | null,
    sfxName: string,
    startFrame: number,
    renderFrame: (frame: number) => Promise<void>
  ): Promise<number> {
    if (!audioEngine || typeof audioEngine.isSoundPlaying !== "function") {
      return 0;
    }
    const shouldTimeout = typeof process !== "undefined" && process.env?.NODE_ENV === "test";
    const maxFrames = shouldTimeout ? 600 : Number.POSITIVE_INFINITY;
    let waited = 0;
    while (audioEngine.isSoundPlaying(sfxName)) {
      if (waited >= maxFrames) {
        throw new Error(`Heal machine wait_sfx timed out waiting for ${sfxName}.`);
      }
      await renderFrame(startFrame + waited);
      waited += 1;
    }
    return waited;
  }

  private awaitHealMachineFrameAsync(): Promise<void> {
    return new Promise((resolve) => {
      if (typeof requestAnimationFrame === "function") {
        requestAnimationFrame(() => resolve());
        return;
      }
      setTimeout(resolve, FRAME_DURATION_MS);
    });
  }

  private determineVisibleBallCount(
    frame: number,
    timeline: HealAnimationTimeline,
    partySlots: number,
    animationType: number
  ): number {
    if (partySlots <= 0) {
      return 0;
    }
    const slots = Math.min(partySlots, this.ballCapacity(animationType));
    return timeline.spawnFrames.slice(0, slots).filter((spawnFrame) => frame >= spawnFrame).length;
  }

  private paletteRotationIndex(frame: number, timeline: HealAnimationTimeline): number {
    const paletteSize = this.loadHealMachinePalette().length;
    if (paletteSize <= 1 || timeline.flashIterations <= 0 || timeline.flashDelay <= 0) {
      return 0;
    }
    if (frame < timeline.flashStart) {
      return 0;
    }
    const flashFrame = frame - timeline.flashStart;
    if (flashFrame >= timeline.flashDuration) {
      return 0;
    }
    // ASM: FlashPalettes rotates the OBJ palette once per toggle, then delays 10 frames.
    const togglesCompleted = Math.floor(flashFrame / timeline.flashDelay) + 1;
    return togglesCompleted % paletteSize;
  }

  private createHealMachineBounds(
    screen: Surface,
    animationType: number,
    anchor: [number, number]
  ): Rect {
    const [width, height] = this.structureSize(animationType);
    const bounds = new Rect(anchor[0], anchor[1], width, height);
    const maxX = screen.get_width() - bounds.width;
    const maxY = screen.get_height() - bounds.height;
    bounds.x = Math.max(0, Math.min(bounds.x, maxX));
    bounds.y = Math.max(0, Math.min(bounds.y, maxY));
    return bounds;
  }

  private structureSize(animationType: number): [number, number] {
    const layout = this.getOamLayout(animationType);
    const [layoutWidth, layoutHeight] = this.layoutDimensions(layout);
    return [layoutWidth, layoutHeight];
  }

  private resolveOamAnchor(animationType: number): [number, number] {
    const layout = this.getOamLayout(animationType);
    const [minX, minY] = this.layoutOrigin(layout);
    const [offsetX, offsetY] = this.resolvePlacementOffset(animationType);
    return [
      minX + offsetX - HealMachineAnimator.OAM_X_BIAS,
      minY + offsetY - HealMachineAnimator.OAM_Y_BIAS,
    ];
  }

  private resolvePlacementOffset(animationType: number): [number, number] {
    return HealMachineAnimator.HEAL_MACHINE_PLACEMENT_OFFSETS[animationType] ?? [0, 0];
  }

  private computeHealMachineSprites(
    animationType: number,
    clampDx: number,
    clampDy: number,
    paletteIndex: number
  ): Array<{ x: number; y: number; surface: Surface }> {
    const layout = this.getOamLayout(animationType);
    const [offsetX, offsetY] = this.resolvePlacementOffset(animationType);
    const tiles = this.loadHealMachineTilesForPalette(paletteIndex);
    const variants = new Map<string, Surface>();
    const positions: Array<{ x: number; y: number; surface: Surface }> = [];
    layout.forEach((entry, index) => {
      // ASM dbsprite stores y first, then x. PlaceHealingMachineTile writes those
      // bytes directly to OAM, whose screen origin is biased by (8, 16).
      const [yTile, xTile, yPx, xPx, tileId, attr] = entry;
      const hardwareX = xTile * TILE_SIZE + xPx + offsetX;
      const hardwareY = yTile * TILE_SIZE + yPx + offsetY;
      const screenX = hardwareX - HealMachineAnimator.OAM_X_BIAS + clampDx;
      const screenY = hardwareY - HealMachineAnimator.OAM_Y_BIAS + clampDy;
      const flipX = Boolean(attr & 0x20);
      const flipY = Boolean(attr & 0x40);
      const surface = this.resolvePokeballSurface(
        tiles,
        tileId,
        flipX,
        flipY,
        variants,
        paletteIndex
      );
      positions.push({ x: screenX, y: screenY, surface });
    });
    return positions;
  }

  private resolvePokeballSurface(
    tiles: Surface[],
    tileId: number,
    flipX: boolean,
    flipY: boolean,
    variants: Map<string, Surface>,
    paletteIndex: number
  ): Surface {
    const key = `${paletteIndex}:${tileId}:${flipX ? 1 : 0}:${flipY ? 1 : 0}`;
    const cached = variants.get(key) ?? HealMachineAnimator.pokeballSurfaces.get(key);
    if (cached) {
      variants.set(key, cached);
      return cached;
    }
    const tileIndex = tileId - HEAL_MACHINE_TILE_BASE;
    if (tileIndex < 0 || tileIndex >= tiles.length) {
      throw new Error(
        `Heal machine tile ${tileId.toString(16)} is out of range for ${tiles.length} tiles.`
      );
    }
    let tile = tiles[tileIndex];
    if (flipX || flipY) {
      tile = gameEngine.transform.flip(tile, flipX, flipY);
    }
    variants.set(key, tile);
    HealMachineAnimator.pokeballSurfaces.set(key, tile);
    return tile;
  }

  private boundedPartySlots(partySlots: number, animationType: number): number {
    const capacity = this.ballCapacity(animationType);
    return Math.max(0, Math.min(MAX_PARTY_SLOTS, capacity, partySlots));
  }

  private ballCapacity(animationType: number): number {
    const layout = this.getOamLayout(animationType);
    const staticCount = HealMachineAnimator.HEAL_MACHINE_STATIC_SPRITES[animationType] ?? 0;
    return Math.max(0, layout.length - staticCount);
  }

  private resolveHealMachineType(animationId: string | null): number {
    if (!animationId) {
      return HealMachineAnimator.HEAL_MACHINE_TYPE_MAP.HEALMACHINE_POKECENTER;
    }
    const lookup = String(animationId).trim().toUpperCase();
    if (!lookup) {
      return HealMachineAnimator.HEAL_MACHINE_TYPE_MAP.HEALMACHINE_POKECENTER;
    }
    if (lookup in HealMachineAnimator.HEAL_MACHINE_TYPE_MAP) {
      return HealMachineAnimator.HEAL_MACHINE_TYPE_MAP[lookup];
    }

    // ASM parity: engine/events/heal_machine_anim.asm indexes a fixed 3-entry pointer table
    // from wScriptVar (HEALMACHINE_POKECENTER/ELMS_LAB/HALL_OF_FAME) and does not clamp.
    const numeric = Number(lookup);
    if (!Number.isInteger(numeric)) {
      throw new Error(`Unknown heal machine animation type '${animationId}'.`);
    }
    if (numeric < 0 || numeric > 2) {
      throw new Error(
        `Heal machine animation type '${animationId}' is out of range; expected 0..2.`,
      );
    }
    return numeric;
  }

  private getOamLayout(
    animationType: number
  ): Array<[number, number, number, number, number, number]> {
    const layout = HealMachineAnimator.HEAL_MACHINE_OAM_LAYOUTS[animationType];
    if (!layout) {
      throw new Error(`Heal machine layout ${animationType} is not defined.`);
    }
    return layout;
  }

  private layoutDimensions(
    layout: Array<[number, number, number, number, number, number]>
  ): [number, number] {
    const [minX, minY] = this.layoutOrigin(layout);
    let maxX = minX;
    let maxY = minY;
    layout.forEach(([yTile, xTile, yPx, xPx]) => {
      const x = xTile * TILE_SIZE + xPx;
      const y = yTile * TILE_SIZE + yPx;
      maxX = Math.max(maxX, x);
      maxY = Math.max(maxY, y);
    });
    return [maxX - minX + TILE_SIZE, maxY - minY + TILE_SIZE];
  }

  private layoutOrigin(
    layout: Array<[number, number, number, number, number, number]>
  ): [number, number] {
    let minX = Number.POSITIVE_INFINITY;
    let minY = Number.POSITIVE_INFINITY;
    layout.forEach(([yTile, xTile, yPx, xPx]) => {
      const x = xTile * TILE_SIZE + xPx;
      const y = yTile * TILE_SIZE + yPx;
      minX = Math.min(minX, x);
      minY = Math.min(minY, y);
    });
    if (!Number.isFinite(minX) || !Number.isFinite(minY)) {
      return [0, 0];
    }
    return [minX, minY];
  }

  private resolveMusicIdentifier(payload: string): string {
    const token = String(payload ?? "").trim();
    if (!token) {
      return "healpokemon";
    }
    return token;
  }

  private loadHealMachineSurface(
    animationType: number,
    paletteIndex: number
  ): Surface | null {
    const key = `${animationType}:${paletteIndex}`;
    const cached = HealMachineAnimator.healMachineSurfaces.get(key);
    if (cached !== undefined) {
      return cached;
    }
    const staticCount = HealMachineAnimator.HEAL_MACHINE_STATIC_SPRITES[animationType] ?? 0;
    if (staticCount <= 0) {
      HealMachineAnimator.healMachineSurfaces.set(key, null);
      return null;
    }
    const tiles = this.loadHealMachineTilesForPalette(paletteIndex);
    const layout = this.getOamLayout(animationType);
    const [minX, minY] = this.layoutOrigin(layout);
    const [width, height] = this.layoutDimensions(layout);
    const surface = this.composeSurfaceFromLayout(tiles, layout, {
      includeIndices: (index) => index < staticCount,
      bounds: { minX, minY, width, height },
    });
    HealMachineAnimator.healMachineSurfaces.set(key, surface);
    return surface;
  }

  private loadHealMachineTiles(): Surface[] {
    if (HealMachineAnimator.healMachineTiles) {
      return HealMachineAnimator.healMachineTiles;
    }
    const tiles = this.loadHealMachineTilesInternal();
    HealMachineAnimator.healMachineTiles = tiles;
    return tiles;
  }

  private loadHealMachineTilesForPalette(paletteIndex: number): Surface[] {
    const cached = HealMachineAnimator.healMachineTilesByPalette.get(paletteIndex);
    if (cached) {
      return cached;
    }
    const baseTiles = this.loadHealMachineTiles();
    const basePalette = this.loadHealMachinePalette();
    const indexedPalette = this.resolveIndexedPalette(baseTiles, basePalette);
    const effectiveBasePalette = indexedPalette ?? basePalette;
    const targetPalette = this.rotatePalette(basePalette, paletteIndex);
    if (paletteIndex === 0 && !indexedPalette) {
      return baseTiles;
    }
    const tiles = this.recolorTiles(baseTiles, effectiveBasePalette, targetPalette);
    HealMachineAnimator.healMachineTilesByPalette.set(paletteIndex, tiles);
    return tiles;
  }

  private loadHealMachineTilesInternal(): Surface[] {
    const canUseFs = typeof window === "undefined";
    if (canUseFs) {
      if (fs.existsSync(HealMachineAnimator.HEAL_MACHINE_2BPP_PATH)) {
        return this.loadHealMachineTilesFrom2bpp();
      }
      if (fs.existsSync(HealMachineAnimator.HEAL_MACHINE_IMAGE_PATH)) {
        return this.loadHealMachineTilesFromPng();
      }
      throw new Error("Heal machine graphics missing: no 2bpp or PNG source found.");
    }
    const loadSync = gameEngine.image.loadSync;
    if (typeof loadSync === "function") {
      const cached = loadSync(HealMachineAnimator.HEAL_MACHINE_IMAGE_PATH);
      if (cached) {
        return this.sliceTilesFromSurface(cached);
      }
    }
    throw new Error(
      "Heal machine graphics must be preloaded before animation in browser environments."
    );
  }

  private loadHealMachineTilesFromPng(): Surface[] {
    const loadSync = gameEngine.image.loadSync;
    if (typeof loadSync !== "function") {
      throw new Error("Heal machine PNG loading requires a synchronous image loader.");
    }
    const surface = loadSync(HealMachineAnimator.HEAL_MACHINE_IMAGE_PATH);
    if (!surface) {
      throw new Error(
        `Heal machine PNG not preloaded: ${HealMachineAnimator.HEAL_MACHINE_IMAGE_PATH}`
      );
    }
    const [width, height] = surface.get_size();
    if (width % TILE_SIZE !== 0 || height % TILE_SIZE !== 0) {
      throw new Error("Heal machine PNG dimensions must be multiples of 8px.");
    }
    return this.sliceTilesFromSurface(surface);
  }

  private loadHealMachineTilesFrom2bpp(): Surface[] {
    const data = fs.readFileSync(HealMachineAnimator.HEAL_MACHINE_2BPP_PATH);
    const palette = this.loadHealMachinePalette();
    if (data.length % 16 !== 0) {
      throw new Error(
        `Heal machine 2bpp data length must be a multiple of 16 bytes (got ${data.length}).`
      );
    }
    return this.decode2bppTiles(data, palette);
  }

  private scaledMachineSurfaceForPalette(
    animationType: number,
    paletteIndex: number,
    bounds: Rect,
    cache: Map<number, Surface | null>
  ): Surface | null {
    if (cache.has(paletteIndex)) {
      return cache.get(paletteIndex) ?? null;
    }
    const surface = this.loadHealMachineSurface(animationType, paletteIndex);
    const scaled = surface ? gameEngine.transform.scale(surface, [bounds.width, bounds.height]) : null;
    cache.set(paletteIndex, scaled);
    return scaled;
  }

  private loadHealMachinePalette(): Array<[number, number, number]> {
    if (HealMachineAnimator.healMachinePalette) {
      return HealMachineAnimator.healMachinePalette;
    }
    const canUseFs = typeof window === "undefined";
    const palette = canUseFs && fs.existsSync(HealMachineAnimator.HEAL_MACHINE_PALETTE_PATH)
      ? this.loadGbPalette(HealMachineAnimator.HEAL_MACHINE_PALETTE_PATH)
      : HealMachineAnimator.HEAL_MACHINE_PALETTE_FALLBACK_5BIT.map(([r, g, b]) => {
        const rgb: [number, number, number] = [
          gbc5To8(r, "heal machine palette r"),
          gbc5To8(g, "heal machine palette g"),
          gbc5To8(b, "heal machine palette b"),
        ];
        return rgb;
      });
    HealMachineAnimator.healMachinePalette = palette;
    return palette;
  }

  private rotatePalette(
    palette: Array<[number, number, number]>,
    rotations: number
  ): Array<[number, number, number]> {
    const entries = palette.length;
    if (entries <= 1) {
      return [...palette];
    }
    const shift = ((rotations % entries) + entries) % entries;
    if (shift === 0) {
      return [...palette];
    }
    return palette.slice(shift).concat(palette.slice(0, shift));
  }

  private recolorTiles(
    tiles: Surface[],
    basePalette: Array<[number, number, number]>,
    targetPalette: Array<[number, number, number]>
  ): Surface[] {
    if (basePalette.length !== targetPalette.length) {
      throw new Error("Heal machine palette rotation requires matching palette lengths.");
    }
    const paletteMap = new Map<string, number>();
    basePalette.forEach((color, index) => {
      paletteMap.set(color.join(","), index);
    });
    return tiles.map((tile, tileIndex) => {
      const recolored = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
      for (let y = 0; y < TILE_SIZE; y += 1) {
        for (let x = 0; x < TILE_SIZE; x += 1) {
          const [r, g, b, a] = tile.get_at([x, y]);
          const key = `${r},${g},${b}`;
          const paletteIndex = paletteMap.get(key);
          if (paletteIndex === undefined) {
            if (a > 0) {
              throw new Error(
                `Heal machine tile ${tileIndex} uses color ${key} outside the palette.`
              );
            }
            recolored.set_at([x, y], [r, g, b, a]);
            continue;
          }
          const [nr, ng, nb] = targetPalette[paletteIndex];
          recolored.set_at([x, y], [nr, ng, nb, a]);
        }
      }
      return recolored;
    });
  }

  private resolveIndexedPalette(
    tiles: Surface[],
    basePalette: Array<[number, number, number]>
  ): Array<[number, number, number]> | null {
    if (basePalette.length !== HealMachineAnimator.HEAL_MACHINE_INDEXED_GRAY_PALETTE.length) {
      return null;
    }
    const baseSet = new Set(basePalette.map((color) => color.join(",")));
    const grayPalette = HealMachineAnimator.HEAL_MACHINE_INDEXED_GRAY_PALETTE;
    const graySet = new Set(grayPalette.map((color) => color.join(",")));
    const observed = new Set<string>();
    for (const tile of tiles) {
      for (let y = 0; y < TILE_SIZE; y += 1) {
        for (let x = 0; x < TILE_SIZE; x += 1) {
          const [r, g, b, a] = tile.get_at([x, y]);
          if (a === 0) {
            continue;
          }
          if (r !== g || r !== b) {
            return null;
          }
          const key = `${r},${g},${b}`;
          observed.add(key);
          if (observed.size > graySet.size) {
            return null;
          }
        }
      }
    }
    if (observed.size === 0) {
      return null;
    }
    let allInBasePalette = true;
    for (const key of observed) {
      if (!baseSet.has(key)) {
        allInBasePalette = false;
      }
      if (!graySet.has(key)) {
        return null;
      }
    }
    if (allInBasePalette) {
      return null;
    }
    // PNG tiles encode palette indices in grayscale; remap to the palette file.
    return grayPalette;
  }

  private composeSurfaceFromLayout(
    tiles: Surface[],
    layout: Array<[number, number, number, number, number, number]>,
    options?: {
      includeIndices?: (index: number) => boolean;
      bounds?: { minX: number; minY: number; width: number; height: number };
    }
  ): Surface {
    if (!layout.length) {
      throw new Error("Heal machine layout is empty; cannot build surface.");
    }
    const tileIds = layout.map((entry) => entry[4]);
    const maxIndex = Math.max(...tileIds);
    const minIndex = Math.min(...tileIds);
    if (minIndex < HEAL_MACHINE_TILE_BASE) {
      throw new Error(
        `Heal machine tile id ${minIndex} precedes base ${HEAL_MACHINE_TILE_BASE.toString(16)}`
      );
    }
    const maxNeeded = maxIndex - HEAL_MACHINE_TILE_BASE;
    if (maxNeeded >= tiles.length) {
      throw new Error(
        `Heal machine data has ${tiles.length} tiles but layout references index ${maxIndex.toString(16)}`
      );
    }

    const [minX, minY] = options?.bounds
      ? [options.bounds.minX, options.bounds.minY]
      : this.layoutOrigin(layout);
    const [width, height] = options?.bounds
      ? [options.bounds.width, options.bounds.height]
      : this.layoutDimensions(layout);
    const includeIndices = options?.includeIndices ?? (() => true);

    const positions: Array<[Surface, number, number]> = [];
    layout.forEach(([yTile, xTile, yPx, xPx, tileId, attr], index) => {
      if (!includeIndices(index)) {
        return;
      }
      const x = xTile * TILE_SIZE + xPx;
      const y = yTile * TILE_SIZE + yPx;
      const tileIndex = tileId - HEAL_MACHINE_TILE_BASE;
      let tile = tiles[tileIndex];
      const flipX = Boolean(attr & 0x20);
      const flipY = Boolean(attr & 0x40);
      if (flipX || flipY) {
        tile = gameEngine.transform.flip(tile, flipX, flipY);
      }
      positions.push([tile, x, y]);
    });

    const surface = new gameEngine.Surface(width, height);
    positions.forEach(([tile, x, y]) => {
      surface.blit(tile, [x - minX, y - minY]);
    });
    return surface;
  }

  private decode2bppTiles(
    data: Buffer,
    palette: Array<[number, number, number]>
  ): Surface[] {
    const tiles: Surface[] = [];
    for (let offset = 0; offset < data.length; offset += 16) {
      const tile = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
      for (let row = 0; row < TILE_SIZE; row += 1) {
        const plane0 = data[offset + row];
        const plane1 = data[offset + row + 8];
        for (let col = 0; col < TILE_SIZE; col += 1) {
          const shift = TILE_SIZE - 1 - col;
          const low = (plane0 >> shift) & 1;
          const high = (plane1 >> shift) & 1;
          const index = (high << 1) | low;
          const color = palette[index] ?? palette[0];
          const alpha = index === 0 ? 0 : 255;
          tile.set_at([col, row], [color[0], color[1], color[2], alpha]);
        }
      }
      tiles.push(tile);
    }
    return tiles;
  }

  private sliceTilesFromSurface(surface: Surface): Surface[] {
    const tiles: Surface[] = [];
    const [width, height] = surface.get_size();
    for (let y = 0; y < height; y += TILE_SIZE) {
      for (let x = 0; x < width; x += TILE_SIZE) {
        const tile = new gameEngine.Surface(TILE_SIZE, TILE_SIZE);
        tile.blit(surface, [0, 0], new Rect(x, y, TILE_SIZE, TILE_SIZE));
        tiles.push(tile);
      }
    }
    return tiles;
  }

  private loadGbPalette(path: string): Array<[number, number, number]> {
    if (!fs.existsSync(path)) {
      throw new Error(`Heal machine palette missing: ${path}`);
    }
    const raw = fs.readFileSync(path, "utf-8");
    const entries: Array<[number, number, number]> = [];
    for (const line of raw.split(/\r?\n/)) {
      const stripped = line.split(";", 1)[0].trim();
      if (!stripped || !stripped.toUpperCase().startsWith("RGB")) {
        continue;
      }
      const parts = stripped.replace(/RGB/i, "").replace(/,/g, " ").trim().split(/\s+/);
      if (parts.length !== 3) {
        throw new Error(`Palette entry must have three components: ${line}`);
      }
      const r = gbc5To8(Number(parts[0]), "heal machine palette r");
      const g = gbc5To8(Number(parts[1]), "heal machine palette g");
      const b = gbc5To8(Number(parts[2]), "heal machine palette b");
      entries.push([r, g, b]);
      if (entries.length >= 4) {
        break;
      }
    }
    if (entries.length < 4) {
      throw new Error(`Heal machine palette ${path} yielded only ${entries.length} entries`);
    }
    return entries;
  }
}
