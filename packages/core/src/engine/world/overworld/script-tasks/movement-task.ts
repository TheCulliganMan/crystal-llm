import { primeWalkStride } from "@pokecrystal/core/engine/systems/animation";
import type { SpriteAnimation } from "@pokecrystal/core/engine/systems/animation";
import { _DIRECTION_VECTORS } from "@pokecrystal/core/engine/world/overworld/constants";
import { LEDGE_JUMP_OFFSETS } from "@pokecrystal/core/engine/world/overworld/jump-offsets";
import { TILE_SIZE } from "@pokecrystal/core/engine/world/tile/constants";
import type { OverworldObject } from "@pokecrystal/core/engine/world/overworld/overworld-object";
import type { OverworldContext } from "@pokecrystal/core/engine/world/story-events/commands/base";
import { ScriptTask } from "./script-task";
import { pushDebugLog } from "@pokecrystal/core/core/debug-log";
import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";

type StepCommandSpeed = "slow" | "walk" | "bike";

type StepCommandProfile = {
  speed: StepCommandSpeed;
  jump: boolean;
};

type MovementTaskOptions = {
  onComplete?: (() => void) | null;
  blocking?: boolean;
  respectCollision?: boolean;
  respectPlayerCollision?: boolean;
};

type MovementScriptFields = {
  object_id?: string | number | null;
  objectId?: string | number | null;
  name?: string;
  x?: number | null;
  y?: number | null;
  prev_x?: number;
  prev_y?: number;
  pixel_x?: number;
  pixelX?: number;
  pixel_y?: number;
  pixelY?: number;
  target_pixel_x?: number;
  targetPixelX?: number;
  target_pixel_y?: number;
  targetPixelY?: number;
  step_dx_px?: number;
  step_dy_px?: number;
  stepDxPx?: number;
  stepDyPx?: number;
  step_frames_remaining?: number;
  stepFramesRemaining?: number;
  step_total_frames?: number;
  stepTotalFrames?: number;
  walking?: boolean;
  jumping?: boolean;
  sliding?: boolean;
  _sliding?: boolean;
  fixed_facing?: string | null;
  _fixed_facing?: string | null;
  direction?: string | null;
  turn?: (direction: string) => void;
  animations?: Record<string, SpriteAnimation>;
  sprite_y_offset?: number;
};

export type MovementTarget = Partial<OverworldObject> & MovementScriptFields;

export type MovementOverworldContext = OverworldContext & {
  WALK_FRAMES?: number;
  TILES_PER_COLLISION?: number;
  leader?: MovementTarget | null;
  _enqueue_follower_step?: (step: string, originX: number, originY: number) => void;
};

export class MovementTask extends ScriptTask<MovementOverworldContext> {
  private static STEP_COMMAND_PROFILES: Record<string, StepCommandProfile> = {
    slow_step: { speed: "slow", jump: false },
    slow_jump_step: { speed: "slow", jump: true },
    slow_slide_step: { speed: "slow", jump: false },
    step: { speed: "walk", jump: false },
    jump_step: { speed: "walk", jump: true },
    slide_step: { speed: "walk", jump: false },
    big_step: { speed: "bike", jump: false },
    fast_jump_step: { speed: "bike", jump: true },
    fast_slide_step: { speed: "bike", jump: false },
  };
  private static TREE_SHAKE_OFFSETS = [0, -1, 1, -1, 1, 0];

  private target: MovementTarget;
  private commands: string[];
  private index = 0;
  private framesRemaining = 0;
  private pendingPixelTarget: [number, number] | null = null;
  private pendingDirection: string | null = null;
  private lastStepDirection: string | null = null;
  private lastStepOrigin: [number, number] | null = null;
  private activeStepIsJump = false;
  private activeJumpHeightPx = 0;
  private activeJumpSpeedPx = 0;
  private treeShakeTarget: MovementTarget | null = null;
  private treeShakeFrame = 0;
  private treeShakeBasePixelX: number | null = null;
  private stationaryFinalSpriteYOffset: number | null = null;
  private _fixed_facing: string | null = null;
  private readonly respectCollision: boolean;
  private readonly respectPlayerCollision: boolean;

  constructor(
    target: MovementTarget,
    commands: string[],
    options: MovementTaskOptions = {},
  ) {
    super({ blocking: options.blocking ?? true, onComplete: options.onComplete ?? null });
    this.target = target;
    this.commands = commands;
    this.respectCollision = options.respectCollision ?? false;
    this.respectPlayerCollision = options.respectPlayerCollision ?? false;
  }

  start(overworld: MovementOverworldContext): void {
    super.start(overworld);
    const trace =
      isDebugEnabled("task:movement") ||
      isDebugEnabled("tasks") ||
      isDebugEnabled("script:tasks") ||
      isDebugEnabled("script");
    if (trace) {
      const identifier = this.target?.object_id ?? this.target?.objectId ?? this.target?.name ?? "?";
      pushDebugLog(`[task] movement start ${identifier}`, { x: this.target?.x, y: this.target?.y });
    }
    this.advance(overworld);
  }

  update(overworld: MovementOverworldContext): void {
    if (this.completed) {
      return;
    }
    if (this.framesRemaining > 0) {
      this.updateTreeShakePosition();
      const stepDx = this.getNumericProp(this.target, "step_dx_px", "stepDxPx");
      const stepDy = this.getNumericProp(this.target, "step_dy_px", "stepDyPx");
      if (stepDx || stepDy) {
        this.offsetPixelPosition(stepDx, stepDy);
      }
      this.updateJumpArc(stepDx, stepDy);
      const remaining = this.getNumericProp(this.target, "step_frames_remaining", "stepFramesRemaining");
      if (remaining > 0) {
        this.setNumericProp("step_frames_remaining", "stepFramesRemaining", Math.max(0, remaining - 1));
      }
      this.framesRemaining -= 1;
      if (this.framesRemaining === 0) {
        this.commitPendingStep(overworld);
        this.advance(overworld);
      }
      return;
    }
    this.advance(overworld);
  }

  finish(overworld: MovementOverworldContext): void {
    this.commitPendingStep(overworld);
    const trace =
      isDebugEnabled("task:movement") ||
      isDebugEnabled("tasks") ||
      isDebugEnabled("script:tasks") ||
      isDebugEnabled("script");
    if (trace) {
      const identifier = this.target?.object_id ?? this.target?.objectId ?? this.target?.name ?? "?";
      pushDebugLog(`[task] movement done ${identifier}`, { x: this.target?.x, y: this.target?.y });
    }
  }

  public getTarget(): MovementTarget | null {
    return this.target ?? null;
  }

  private advance(overworld: MovementOverworldContext): void {
    while (this.index < this.commands.length) {
      const raw = this.commands[this.index];
      this.index += 1;
      if (!raw) {
        continue;
      }
      const parts = raw.split(/\s+/).filter(Boolean);
      if (!parts.length) {
        continue;
      }
      const instruction = parts[0].toLowerCase();
      const stepProfile = MovementTask.STEP_COMMAND_PROFILES[instruction];
      if (stepProfile) {
        if (parts.length < 2) {
          throw new Error(`Movement command '${instruction}' requires a direction`);
        }
        const direction = parts[1].toLowerCase();
        this.scheduleStep(direction, stepProfile, instruction, overworld);
        return;
      }
      if (instruction === "turn_head") {
        if (parts.length >= 2) {
          this.setDirection(parts[1].toLowerCase());
        }
        // ASM: engine/overworld/movement.asm::Movement_turn_head_* only updates facing; treat as a 1-frame command.
        this.framesRemaining = 1;
        if ("walking" in this.target) {
          this.target.walking = false;
        }
        if ("jumping" in this.target) {
          this.target.jumping = false;
        }
        if (this.framesRemaining) {
          return;
        }
        continue;
      }
      if (instruction === "step_sleep") {
        let duration = overworld.WALK_FRAMES ?? 1;
        if (parts.length >= 2) {
          const parsed = Number(parts[1]);
          if (!Number.isFinite(parsed)) {
            throw new Error(`Invalid step_sleep duration '${parts[1]}'`);
          }
          duration = Math.trunc(parsed);
        }
        this.framesRemaining = Math.max(1, duration);
        return;
      }
      if (instruction === "tree_shake") {
        this.stationaryFinalSpriteYOffset = null;
        this.startTreeShake();
        return;
      }
      if (instruction === "fix_facing") {
        this.lockFacing();
        continue;
      }
      if (instruction === "remove_fixed_facing") {
        this.unlockFacing();
        continue;
      }
      if (instruction === "set_sliding") {
        this.setSliding(true);
        continue;
      }
      if (instruction === "remove_sliding") {
        this.setSliding(false);
        continue;
      }
      if (instruction === "teleport_from") {
        // ASM: StepFunction_TeleportFrom spins for 8 frames, then rises for 16.
        this.scheduleStationaryEffect(24, 0);
        return;
      }
      if (instruction === "teleport_to") {
        // ASM: StepFunction_TeleportTo waits 16, descends 16, then spins for 32.
        this.scheduleStationaryEffect(64, 0);
        return;
      }
      if (instruction === "skyfall_top") {
        // ASM: StepFunction_SkyfallTop runs OBJECT_ACTION_SKYFALL for 16 frames,
        // then leaves the sprite at y offset $60 until the following warp/script work.
        this.scheduleStationaryEffect(16, 0x60);
        return;
      }
      if (instruction === "step_end") {
        this.completed = true;
        return;
      }
      throw new Error(`Unsupported movement command '${instruction}'`);
    }
    this.completed = true;
  }

  private scheduleStep(
    direction: string,
    profile: StepCommandProfile,
    instruction: string,
    overworld: MovementOverworldContext
  ): void {
    this.stationaryFinalSpriteYOffset = null;
    this.lastStepDirection = direction;
    const vector = _DIRECTION_VECTORS[direction] ?? [0, 0];
    const [dx, dy] = vector;
    if (dx === 0 && dy === 0) {
      throw new Error(`Unsupported movement direction '${direction}'`);
    }
    const stride = Math.max(1, Math.trunc(overworld.TILES_PER_COLLISION ?? 1));
    // ASM: JumpStep runs jump + land phases (map_objects.asm::StepFunction_*Jump),
    // so scripted jump steps advance two tiles at the same per-phase speed.
    const stepStride = profile.jump ? stride * 2 : stride;
    const originX = this.target?.x;
    const originY = this.target?.y;
    if (originX === undefined || originY === undefined || originX === null || originY === null) {
      throw new Error("Movement targets must expose tile coordinates.");
    }
    this.lastStepOrigin = [Math.trunc(originX), Math.trunc(originY)];
    const targetX = originX + dx * stepStride;
    const targetY = originY + dy * stepStride;
    const blocker = (overworld as any)._npc_step_blocked;
    const isPlayerTarget = this.isPlayerTarget();
    if (this.respectPlayerCollision && !isPlayerTarget && typeof blocker === "function") {
      const blockedByPlayer = Boolean(blocker.call(overworld, this.target, direction, targetX, targetY, {
        player_only: true,
        suppress_blocked_log: true,
      }));
      if (blockedByPlayer) {
        this.completed = true;
        return;
      }
    }
    // ASM applymovement advances scripted actors even when terrain or NPC probes would report blocked;
    // keep the probe for occupancy/collision parity, but suppress blocker debug noise for cutscenes.
    const probeOptions: { is_player_target?: boolean; suppress_blocked_log: boolean } = {
      suppress_blocked_log: true,
    };
    if (isPlayerTarget) {
      probeOptions.is_player_target = true;
    }
    const blocked = Boolean(blocker?.call(overworld, this.target, direction, targetX, targetY, probeOptions));
    if (blocked && this.respectCollision) {
      this.completed = true;
      return;
    }
    const { duration, pixelsPerFrame, totalPixels } = this.resolveStepProfile(
      profile.speed,
      profile.jump,
      stepStride,
      overworld,
    );
    const stepDxPx = dx * pixelsPerFrame;
    const stepDyPx = dy * pixelsPerFrame;
    const startPixelX = this.getNumericProp(this.target, "pixel_x", "pixelX");
    const startPixelY = this.getNumericProp(this.target, "pixel_y", "pixelY");
    const facingDirection = this.facingLockedDirection() ?? direction;
    this.prepareStep(
      targetX,
      targetY,
      facingDirection,
      duration,
      stepDxPx,
      stepDyPx,
      startPixelX + dx * totalPixels,
      startPixelY + dy * totalPixels,
      profile.jump,
    );

    if (
      this.lastStepDirection &&
      this.lastStepOrigin &&
      overworld.leader &&
      overworld.leader === this.target
    ) {
      const [originX2, originY2] = this.lastStepOrigin;
      overworld._enqueue_follower_step?.(`${instruction} ${this.lastStepDirection}`, originX2, originY2);
    }
  }

  private resolveStepProfile(
    speed: StepCommandSpeed,
    isJump: boolean,
    stepStride: number,
    overworld: MovementOverworldContext
  ): { duration: number; pixelsPerFrame: number; totalPixels: number } {
    const walkFrames = Math.max(1, Math.trunc(overworld.WALK_FRAMES ?? 1));
    let baseDuration = walkFrames;
    if (speed === "slow") {
      baseDuration = walkFrames * 2;
    } else if (speed === "bike") {
      baseDuration = Math.max(1, Math.trunc(walkFrames / 2));
    }
    const duration = baseDuration * (isJump ? 2 : 1);
    const totalPixels = Math.max(1, Math.trunc(stepStride * TILE_SIZE));
    if (duration <= 0) {
      throw new Error("Movement duration must be positive.");
    }
    const pixelsPerFrame = totalPixels / duration;
    return { duration, pixelsPerFrame, totalPixels };
  }

  private startTreeShake(): void {
    this.treeShakeTarget = this.target;
    this.treeShakeFrame = 0;
    this.treeShakeBasePixelX = this.getNumericProp(this.target, "pixel_x", "pixelX");
    if ("walking" in this.target) {
      this.target.walking = true;
    }
    if ("jumping" in this.target) {
      this.target.jumping = false;
    }
    this.framesRemaining = Math.max(1, 24);
  }

  private scheduleStationaryEffect(durationFrames: number, finalSpriteYOffset: number): void {
    this.commitPendingStepSilently();
    this.stationaryFinalSpriteYOffset = finalSpriteYOffset;
    if ("walking" in this.target) {
      this.target.walking = false;
    }
    if ("jumping" in this.target) {
      this.target.jumping = false;
    }
    this.setNumericProp("step_dx_px", "stepDxPx", 0);
    this.setNumericProp("step_dy_px", "stepDyPx", 0);
    this.setNumericProp("step_frames_remaining", "stepFramesRemaining", Math.max(1, Math.trunc(durationFrames)));
    this.setNumericProp("step_total_frames", "stepTotalFrames", Math.max(1, Math.trunc(durationFrames)));
    this.framesRemaining = Math.max(1, Math.trunc(durationFrames));
  }

  private updateTreeShakePosition(): void {
    if (!this.treeShakeTarget || this.treeShakeBasePixelX === null) {
      return;
    }
    const offsetIndex = this.treeShakeFrame % MovementTask.TREE_SHAKE_OFFSETS.length;
    const offset = MovementTask.TREE_SHAKE_OFFSETS[offsetIndex];
    const pixelX = this.treeShakeBasePixelX + offset;
    this.setNumericProp("pixel_x", "pixelX", pixelX, this.treeShakeTarget);
    this.setNumericProp("target_pixel_x", "targetPixelX", pixelX, this.treeShakeTarget);
    this.treeShakeFrame += 1;
  }

  private updateJumpArc(stepDxPx: number, stepDyPx: number): void {
    if (!this.activeStepIsJump) {
      if (this.activeJumpSpeedPx) {
        this.setSpriteYOffset(0);
      }
      return;
    }

    if (!this.activeJumpSpeedPx) {
      const totalFrames = Math.max(
        1,
        Math.trunc(this.getNumericProp(this.target, "step_total_frames", "stepTotalFrames") || 1),
      );
      this.activeJumpSpeedPx = this.resolveJumpSpeed(stepDxPx, stepDyPx, totalFrames);
    }

    // ASM: UpdateJumpPosition (map_objects.asm) indexes the 16-entry table by OBJECT_JUMP_HEIGHT >> 1.
    const jumpIndex = Math.trunc(this.activeJumpHeightPx / 2);
    const maxIndex = LEDGE_JUMP_OFFSETS.length - 1;
    if (jumpIndex < 0 || jumpIndex > maxIndex) {
      throw new Error(
        `Jump step height index ${jumpIndex} out of range for UpdateJumpPosition (${this.activeJumpHeightPx}).`,
      );
    }
    this.setSpriteYOffset(LEDGE_JUMP_OFFSETS[jumpIndex]);
    this.activeJumpHeightPx += this.activeJumpSpeedPx;
  }

  private resolveJumpSpeed(stepDxPx: number, stepDyPx: number, totalFrames: number): number {
    const stridePx = Math.max(Math.abs(stepDxPx), Math.abs(stepDyPx));
    if (!Number.isFinite(stridePx) || stridePx <= 0) {
      throw new Error("Jump step requires a non-zero step vector.");
    }
    const rounded = Math.round(stridePx);
    if (Math.abs(stridePx - rounded) > 1e-6) {
      throw new Error(`Jump step speed ${stridePx} is not ASM-aligned.`);
    }
    const expectedHeight = rounded * Math.max(1, Math.trunc(totalFrames));
    const expectedMax = LEDGE_JUMP_OFFSETS.length * 2;
    if (expectedHeight !== expectedMax) {
      throw new Error(
        `Jump step timing mismatch: speed ${rounded} * frames ${totalFrames} = ${expectedHeight}, expected ${expectedMax}.`,
      );
    }
    return rounded;
  }

  private setSpriteYOffset(offset: number): void {
    if ("sprite_y_offset" in this.target) {
      this.target.sprite_y_offset = Number.isFinite(offset) ? offset : Number(offset);
    }
  }

  private prepareStep(
    targetX: number,
    targetY: number,
    direction: string,
    durationFrames: number,
    stepDxPx: number,
    stepDyPx: number,
    targetPixelX: number,
    targetPixelY: number,
    isJump: boolean,
  ): void {
    const originX = this.target?.x ?? targetX;
    const originY = this.target?.y ?? targetY;
    this.setPrevCoords(originX, originY);
    this.setDirection(direction);
    this.primeStride(direction);
    this.activeStepIsJump = isJump;
    this.activeJumpHeightPx = 0;
    this.activeJumpSpeedPx = isJump
      ? this.resolveJumpSpeed(stepDxPx, stepDyPx, durationFrames)
      : 0;
    this.pendingDirection = direction;
    if ("x" in this.target) {
      this.target.x = targetX;
    }
    if ("y" in this.target) {
      this.target.y = targetY;
    }
    if (this.hasPixelCoordinates(this.target)) {
      this.setNumericProp("step_dx_px", "stepDxPx", stepDxPx);
      this.setNumericProp("step_dy_px", "stepDyPx", stepDyPx);
      this.setNumericProp("step_frames_remaining", "stepFramesRemaining", durationFrames);
      this.setNumericProp("step_total_frames", "stepTotalFrames", durationFrames);
      if (isJump) {
        this.target.jumping = true;
        if ("walking" in this.target) {
          this.target.walking = false;
        }
      } else {
        this.target.walking = true;
        if ("jumping" in this.target) {
          this.target.jumping = false;
        }
      }
      this.pendingPixelTarget = [targetPixelX, targetPixelY];
    }
    this.setSpriteYOffset(0);
    this.framesRemaining = Math.max(1, Math.trunc(durationFrames));
  }

  private commitPendingStep(overworld: MovementOverworldContext): void {
    if (this.pendingPixelTarget && this.hasPixelCoordinates(this.target)) {
      const [targetPixelX, targetPixelY] = this.pendingPixelTarget;
      const pixelX = Math.trunc(Math.round(targetPixelX));
      const pixelY = Math.trunc(Math.round(targetPixelY));
      this.setNumericProp("pixel_x", "pixelX", pixelX);
      this.setNumericProp("pixel_y", "pixelY", pixelY);
      this.setNumericProp("target_pixel_x", "targetPixelX", pixelX);
      this.setNumericProp("target_pixel_y", "targetPixelY", pixelY);
      this.setNumericProp("step_dx_px", "stepDxPx", 0);
      this.setNumericProp("step_dy_px", "stepDyPx", 0);
      this.setNumericProp("step_frames_remaining", "stepFramesRemaining", 0);
      this.setNumericProp("step_total_frames", "stepTotalFrames", 0);
      this.target.walking = false;
      this.target.jumping = false;
    } else {
      if ("walking" in this.target) {
        this.target.walking = false;
      }
      if ("jumping" in this.target) {
        this.target.jumping = false;
      }
    }
    const finalX = this.target?.x;
    const finalY = this.target?.y;
    if (finalX !== undefined && finalY !== undefined) {
      this.setPrevCoords(finalX, finalY);
    }
    this.setSpriteYOffset(0);
    if (this.stationaryFinalSpriteYOffset !== null) {
      this.setSpriteYOffset(this.stationaryFinalSpriteYOffset);
    }
    if (this.pendingDirection) {
      this.setDirection(this.pendingDirection);
    }
    this.pendingDirection = null;
    this.pendingPixelTarget = null;
    this.activeStepIsJump = false;
    this.activeJumpHeightPx = 0;
    this.activeJumpSpeedPx = 0;
    this.lastStepOrigin = null;
    this.clearTreeShake();
  }

  private commitPendingStepSilently(): void {
    if (!this.pendingPixelTarget && !this.treeShakeTarget) {
      return;
    }
    this.commitPendingStep({} as MovementOverworldContext);
  }

  private clearTreeShake(): void {
    const target = this.treeShakeTarget;
    if (!target) {
      return;
    }
    if (this.treeShakeBasePixelX !== null && this.hasPixelCoordinates(target)) {
      this.setNumericProp("pixel_x", "pixelX", this.treeShakeBasePixelX, target);
      this.setNumericProp("target_pixel_x", "targetPixelX", this.treeShakeBasePixelX, target);
    }
    if ("walking" in target) {
      target.walking = false;
    }
    this.treeShakeTarget = null;
    this.treeShakeBasePixelX = null;
    this.treeShakeFrame = 0;
  }

  private primeStride(direction: string): void {
    const animations = this.target?.animations;
    if (!animations || typeof animations !== "object") {
      return;
    }
    const animation = animations[direction] ?? animations["down"];
    primeWalkStride(animation ?? null);
  }

  private setDirection(direction: string): void {
    if (this.isFacingLocked()) {
      return;
    }
    if (typeof this.target?.turn === "function") {
      try {
        this.target.turn(direction);
        return;
      } catch {
        // Fall back to assignment.
      }
    }
    if ("direction" in this.target) {
      this.target.direction = direction;
    }
  }

  private hasPixelCoordinates(target: MovementTarget | null): boolean {
    if (!target) {
      return false;
    }
    return "pixel_x" in target || "pixelX" in target;
  }

  private isPlayerTarget(): boolean {
    const targetRecord = this.target as Record<string, unknown> | null;
    if (!targetRecord) {
      return false;
    }
    const identifiers = [
      targetRecord.object_id,
      targetRecord.objectId,
      targetRecord.name,
      targetRecord.constantId,
    ];
    return identifiers.some((value) => String(value ?? "").toUpperCase() === "PLAYER");
  }

  private getNumericProp(target: MovementTarget | null, snake: string, camel: string): number {
    if (!target) {
      return 0;
    }
    const targetRecord = target as Record<string, unknown>;
    const value =
      (snake in targetRecord ? targetRecord[snake] : undefined) ??
      (camel in targetRecord ? targetRecord[camel] : undefined);
    const numeric = Number(value ?? 0);
    return Number.isFinite(numeric) ? numeric : 0;
  }

  private setNumericProp(
    snake: string,
    camel: string,
    value: number,
    target: MovementTarget | null = this.target,
  ): void {
    if (!target) {
      return;
    }
    const targetRecord = target as Record<string, unknown>;
    if (snake in targetRecord || !(camel in targetRecord)) {
      targetRecord[snake] = value;
    }
    if (camel in targetRecord || !(snake in targetRecord)) {
      targetRecord[camel] = value;
    }
  }

  private setPrevCoords(x: number, y: number): void {
    if (!this.target) {
      return;
    }
    const numericX = Math.trunc(Number(x));
    const numericY = Math.trunc(Number(y));
    if (!Number.isFinite(numericX) || !Number.isFinite(numericY)) {
      return;
    }
    this.target.prev_x = numericX;
    this.target.prev_y = numericY;
    this.target.prevX = numericX;
    this.target.prevY = numericY;
  }

  private setSliding(sliding: boolean): void {
    const targetRecord = this.target as Record<string, unknown>;
    if ("sliding" in targetRecord || !("_sliding" in targetRecord)) {
      this.target.sliding = sliding;
    }
    if ("_sliding" in targetRecord) {
      this.target._sliding = sliding;
    }
  }

  private offsetPixelPosition(dx: number, dy: number): void {
    if (!this.target) {
      return;
    }
    const pixelX = this.getNumericProp(this.target, "pixel_x", "pixelX");
    const pixelY = this.getNumericProp(this.target, "pixel_y", "pixelY");
    this.setNumericProp("pixel_x", "pixelX", pixelX + dx);
    this.setNumericProp("pixel_y", "pixelY", pixelY + dy);
  }

  private facingLockedDirection(): string | null {
    if ("fixed_facing" in this.target) {
      const locked = this.target.fixed_facing;
      if (typeof locked === "string") {
        return locked;
      }
      if (locked) {
        return this.target.direction ?? null;
      }
    }
    if ("_fixed_facing" in this.target) {
      const locked = this.target._fixed_facing;
      if (locked) {
        return locked;
      }
    }
    if (this._fixed_facing) {
      return this._fixed_facing;
    }
    return null;
  }

  private isFacingLocked(): boolean {
    return this.facingLockedDirection() !== null;
  }

  private lockFacing(): void {
    const currentDirection = this.target?.direction ?? null;
    if ("fixed_facing" in this.target) {
      this.target.fixed_facing = currentDirection;
      return;
    }
    if ("_fixed_facing" in this.target) {
      this.target._fixed_facing = currentDirection;
      return;
    }
    this._fixed_facing = currentDirection;
  }

  private unlockFacing(): void {
    if ("fixed_facing" in this.target) {
      this.target.fixed_facing = null;
    }
    if ("_fixed_facing" in this.target) {
      this.target._fixed_facing = null;
    }
    if (this._fixed_facing) {
      this._fixed_facing = null;
    }
  }
}
