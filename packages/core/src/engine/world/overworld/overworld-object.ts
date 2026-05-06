import { ObjectEvent } from "../../../core/models/map";
import { SpriteAnimation } from "../../systems/animation";
import { METATILE_SIZE, METATILE_WIDTH, TILE_SIZE } from "../tile";

/**
 * Convert script or object identifiers into the command format.
 * @param scriptName - The script or object identifier.
 * @returns The normalized identifier.
 */
function normaliseObjectIdentifier(scriptName: string | null): string | null {
  if (!scriptName) {
    return null;
  }
  let base = scriptName.trim();
  if (base.endsWith("Script")) {
    base = base.slice(0, -6);
  }
  base = base.replace(/\./g, "_");
  base = base.replace(/([a-z0-9])([A-Z])/g, "$1_$2");
  base = base.replace(/__+/g, "_");
  return base.toUpperCase();
}

export class OverworldObject {
  event: ObjectEvent;
  baseSprite: string;
  baseSpriteId: string;
  spriteConstant: string;
  name: string;
  private _x: number;
  private _y: number;
  private _prevX: number;
  private _prevY: number;
  private _direction: string;
  facing: number; // STANDING
  objectId: string | null;
  constantId: string | null;
  spriteId: string;
  animations: Record<string, SpriteAnimation>;
  objectIndex: number;
  palette: number;
  initialSubtileX: number;
  initialSubtileY: number;
  protected _collisionStride: number;
  protected _footprint: number;
  private _pixelX: number;
  private _pixelY: number;
  spriteYOffset: number;
  private _targetPixelX: number;
  private _targetPixelY: number;
  private _stepDxPx: number;
  private _stepDyPx: number;
  stepFramesRemaining: number;
  stepTotalFrames: number;
  walking: boolean;
  jumping: boolean;
  overhead: boolean; // OVERHEAD_F flag - sprite draws below priority tiles
  fixedFacing: string | null; // Mirrors the ASM FIXED_FACING_F flag
  constructor(event: ObjectEvent, overworld?: any) {
    this.event = event;
    this.baseSprite = event.sprite;
    this.baseSpriteId = this.baseSprite.replace("SPRITE_", "").toUpperCase();
    this.spriteConstant = event.sprite;
    this.name = this.spriteConstant.replace("SPRITE_", "");
    this._x = event.x;
    this._y = event.y;
    this._prevX = event.x;
    this._prevY = event.y;
    this._direction = "down";
    this.facing = -1; // STANDING
    this.objectId = normaliseObjectIdentifier(event.script);
    this.constantId = event.object_identifier
      ? String(event.object_identifier).trim().toUpperCase()
      : null;
    this.spriteId = this.name.toUpperCase();
    this.animations = {};
    this.objectIndex = 0;
    this.palette = event.pal;
    this.initialSubtileX = 0;
    this.initialSubtileY = 0;
    this._collisionStride = 2;
    this._footprint = this._collisionStride - 1;
    this._pixelX = 0;
    this._pixelY = 0;
    this.spriteYOffset = 0.0;
    this._targetPixelX = 0;
    this._targetPixelY = 0;
    this._stepDxPx = 0;
    this._stepDyPx = 0;
    this.stepFramesRemaining = 0;
    this.stepTotalFrames = 0;
    this.walking = false;
    this.jumping = false;
    this.overhead = false;
    this.fixedFacing = null;
  }

  /**
   * Update the object's active sprite while preserving the original label.
   * @param spriteConstant - The new sprite constant.
   */
  setSprite(spriteConstant: string): void {
    this.spriteConstant = spriteConstant;
    this.name = spriteConstant.replace("SPRITE_", "");
    this.spriteId = this.name.toUpperCase();
  }

  /**
   * Turn the object in a given direction.
   * @param direction - The direction to turn.
   * @param force - Whether to force the turn even if facing is fixed.
   */
  turn(direction: string, force: boolean = false): void {
    if (this.fixedFacing !== null && !force) {
      return;
    }
    const lowerDir = direction.toLowerCase();
    if (["up", "down", "left", "right"].includes(lowerDir)) {
      this.direction = lowerDir;
    }
  }

  /**
   * Turn the NPC toward the player's tile coordinates.
   * @param playerX - The player's x coordinate.
   * @param playerY - The player's y coordinate.
   */
  facePlayer(playerX: number, playerY: number): void {
    if (this.fixedFacing !== null) {
      return;
    }
    const dx = playerX - this.x;
    const dy = playerY - this.y;
    if (dx === 0 && dy === 0) {
      return;
    }

    if (Math.abs(dy) >= Math.abs(dx)) {
      this.direction = dy > 0 ? "down" : "up";
    } else {
      this.direction = dx > 0 ? "right" : "left";
    }
  }

  private static tileToPixels(tileCoordinate: number): number {
    const metatile = Math.floor(tileCoordinate / METATILE_WIDTH);
    const offset = tileCoordinate % METATILE_WIDTH;
    return metatile * METATILE_SIZE + offset * TILE_SIZE;
  }

  setCollisionStride(stride: number): void {
    this.collisionStride = Math.max(1, stride);
    this.footprint = this.collisionStride - 1;
  }

  updatePixelPosition(): void {
    const originX = this.x - this.footprint;
    const originY = this.y - this.footprint;
    this.pixelX = OverworldObject.tileToPixels(originX);
    this.pixelY = OverworldObject.tileToPixels(originY);
    this.targetPixelX = this.pixelX;
    this.targetPixelY = this.pixelY;
    this.spriteYOffset = 0.0;
  }

  get x(): number {
    return this._x;
  }

  set x(value: number) {
    this._x = value;
  }

  get y(): number {
    return this._y;
  }

  set y(value: number) {
    this._y = value;
  }

  get prevX(): number {
    return this._prevX;
  }

  set prevX(value: number) {
    this._prevX = value;
  }

  get prevY(): number {
    return this._prevY;
  }

  set prevY(value: number) {
    this._prevY = value;
  }

  get direction(): string {
    return this._direction;
  }

  set direction(value: string) {
    this._direction = value;
  }

  get collisionStride(): number {
    return this._collisionStride;
  }

  set collisionStride(value: number) {
    this._collisionStride = value;
  }

  get footprint(): number {
    return this._footprint;
  }

  set footprint(value: number) {
    this._footprint = value;
  }

  get pixelX(): number {
    return this._pixelX;
  }

  set pixelX(value: number) {
    this._pixelX = value;
  }

  get pixel_x(): number {
    return this._pixelX;
  }

  set pixel_x(value: number) {
    this._pixelX = value;
  }

  get pixelY(): number {
    return this._pixelY;
  }

  set pixelY(value: number) {
    this._pixelY = value;
  }

  get pixel_y(): number {
    return this._pixelY;
  }

  set pixel_y(value: number) {
    this._pixelY = value;
  }

  get targetPixelX(): number {
    return this._targetPixelX;
  }

  set targetPixelX(value: number) {
    this._targetPixelX = value;
  }

  get target_pixel_x(): number {
    return this._targetPixelX;
  }

  set target_pixel_x(value: number) {
    this._targetPixelX = value;
  }

  get targetPixelY(): number {
    return this._targetPixelY;
  }

  set targetPixelY(value: number) {
    this._targetPixelY = value;
  }

  get target_pixel_y(): number {
    return this._targetPixelY;
  }

  set target_pixel_y(value: number) {
    this._targetPixelY = value;
  }

  get sprite_y_offset(): number {
    return this.spriteYOffset;
  }

  set sprite_y_offset(value: number) {
    this.spriteYOffset = value;
  }

  get stepDxPx(): number {
    return this._stepDxPx;
  }

  set stepDxPx(value: number) {
    this._stepDxPx = value;
  }

  get stepDyPx(): number {
    return this._stepDyPx;
  }

  set stepDyPx(value: number) {
    this._stepDyPx = value;
  }

  /**
   * Update the OVERHEAD_F flag based on current tile collision.
   * @param collisionValue - The collision value of the current tile.
   */
  updateOverheadFlag(collisionValue: number): void {
    if ((collisionValue & 0xf0) === 0x10) {
      const lowNibble = collisionValue & 0x0f;
      this.overhead =
        [0x4, 0x8, 0x9, 0xa, 0xb, 0xc].includes(lowNibble) ||
        [0x14, 0x18, 0x1c].includes(collisionValue);
    } else {
      this.overhead = false;
    }
  }

  /**
   * Applies a sequence of movements to the object.
   * @param movementData - A list of movement commands.
   */
  applyMovement(movementData: string[]): void {
    const stride = 2;
    const jumpStride = stride * 2;
    const stepLengths: Record<string, number> = {
      slow_step: stride,
      slow_jump_step: jumpStride,
      slow_slide_step: stride,
      step: stride,
      jump_step: jumpStride,
      slide_step: stride,
      big_step: stride,
      fast_jump_step: jumpStride,
      fast_slide_step: stride,
    };

    const applyStep = (direction: string, distance: number) => {
      direction = direction.toLowerCase();
      if (direction === "left") {
        this.prevX = this.x;
        this.x -= distance;
        if (this.fixedFacing === null) this.direction = "left";
      } else if (direction === "right") {
        this.prevX = this.x;
        this.x += distance;
        if (this.fixedFacing === null) this.direction = "right";
      } else if (direction === "up") {
        this.prevY = this.y;
        this.y -= distance;
        if (this.fixedFacing === null) this.direction = "up";
      } else if (direction === "down") {
        this.prevY = this.y;
        this.y += distance;
        if (this.fixedFacing === null) this.direction = "down";
      }
    };

    for (const command of movementData) {
      const parts = command.split(" ");
      if (parts.length === 0) continue;
      const move = parts[0];
      if (move in stepLengths && parts.length > 1) {
        applyStep(parts[1], stepLengths[move]);
      } else if (move === "turn_head" && parts.length > 1) {
        if (this.fixedFacing === null) {
          this.direction = parts[1].toLowerCase();
        }
      } else if (move === "fix_facing") {
        this.fixedFacing = this.direction;
      } else if (move === "remove_fixed_facing") {
        this.fixedFacing = null;
      } else if (move === "step_sleep") {
        continue;
      } else if (move === "step_end") {
        break;
      }
    }
    this.updatePixelPosition();
    // ASM: CopyCoordsTileToLastCoordsTile after movement completion.
    this.prevX = this.x;
    this.prevY = this.y;
    (this as { prev_x?: number; prev_y?: number }).prev_x = this.prevX;
    (this as { prev_x?: number; prev_y?: number }).prev_y = this.prevY;
  }

  /**
   * Check if the object matches a given identifier.
   * @param identifier - The identifier to check.
   * @param map_name - The name of the map to check against.
   * @returns True if the object matches the identifier.
   */
  matchesIdentifier(identifier: string, map_name: string | null = null): boolean {
    const normalised = identifier.toUpperCase();
    if (String(this.objectIndex) === normalised) {
      return true;
    }
    const spriteId = this.spriteId.toUpperCase();
    if (spriteId === normalised) {
      return true;
    }
    const baseSpriteId = this.baseSpriteId.toUpperCase();
    if (baseSpriteId === normalised) {
      return true;
    }
    if (this.objectId && this.objectId.toUpperCase() === normalised) {
      return true;
    }
    if (this.constantId && this.constantId.toUpperCase() === normalised) {
      return true;
    }

    if (map_name) {
      const mapKey = map_name.replace(/\s+/g, "").toUpperCase();
      if (`${mapKey}_${spriteId}` === normalised) {
        return true;
      }
      if (`${mapKey}_${baseSpriteId}` === normalised) {
        return true;
      }
      if (this.objectIndex) {
        if (`${mapKey}_${spriteId}${this.objectIndex}` === normalised) {
          return true;
        }
        if (`${mapKey}_${baseSpriteId}${this.objectIndex}` === normalised) {
          return true;
        }
      }
      if (this.objectId && `${mapKey}_${this.objectId.toUpperCase()}` === normalised) {
        return true;
      }
    }
    return false;
  }
}
