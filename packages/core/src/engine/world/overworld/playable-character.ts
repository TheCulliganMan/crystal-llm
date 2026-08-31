import { PlayerState } from "@pokecrystal/core/core/enums/overworld";
import type { Overworld } from "@pokecrystal/core/types/overworld";
import { OverworldObject } from "./overworld-object";
import { ObjectEvent } from "@pokecrystal/core/core/models/map";

type PlayerFieldKey = keyof Pick<
  Overworld,
  | "player_x"
  | "player_y"
  | "prev_player_x"
  | "prev_player_y"
  | "player_px_x"
  | "player_px_y"
  | "target_px_x"
  | "target_px_y"
  | "step_dx_px"
  | "step_dy_px"
>;

// ASM reference: engine/overworld/movement.asm (scripted movement proxy for the player).
export class PlayerCharacter extends OverworldObject {
  private static readonly STEP_COMMANDS: Record<string, number> = {
    step: 2,
    slow_step: 2,
    slow_jump_step: 4,
    jump_step: 4,
    fast_jump_step: 4,
    big_step: 2,
    slow_slide_step: 2,
    slide_step: 2,
    fast_slide_step: 2,
  };

  private _overworld: Overworld | null = null;
  private _pendingPlayerFieldValues?: Partial<Record<PlayerFieldKey, number>>;
  private _pendingDirection: string | null = null;
  public readonly name = "PLAYER";
  private _fixed_facing: string | null = null;

  public walking = false;
  public jumping = false;
  public step_frames_remaining = 0;
  public step_total_frames = 0;
  public overhead = false;

  constructor(overworld: Overworld) {
    super(
      ObjectEvent.parse({
        sprite: "PLAYER",
        sprite_has_facings: true,
        x: 0,
        y: 0,
        pal: 0,
        script: "Player",
        spritemovedata: "",
        move_range_x: 0,
        move_range_y: 0,
        hram_x: 0,
        hram_y: 0,
        object_type: "",
        radius: 0,
        event_flag: "",
      }),
      overworld
    );
    this._overworld = overworld;
    this._flushPendingPlayerFieldValues();
    this._flushPendingDirection();
  }

  private get overworld(): Overworld {
    if (!this._overworld) {
      throw new Error("PlayerCharacter bound before Overworld initialization");
    }
    return this._overworld;
  }

  private _setPlayerField(field: PlayerFieldKey, value: number): void {
    if (this._overworld) {
      this._overworld[field] = value;
      return;
    }
    if (!this._pendingPlayerFieldValues) {
      this._pendingPlayerFieldValues = {};
    }
    this._pendingPlayerFieldValues[field] = value;
  }

  private _flushPendingPlayerFieldValues(): void {
    if (!this._overworld || !this._pendingPlayerFieldValues) {
      return;
    }
    for (const [key, value] of Object.entries(this._pendingPlayerFieldValues)) {
      const playerField = key as PlayerFieldKey;
      if (value === undefined) {
        continue;
      }
      if (this._overworld[playerField] === undefined || this._overworld[playerField] === null) {
        this._overworld[playerField] = value;
      }
    }
    this._pendingPlayerFieldValues = undefined;
  }

  private _flushPendingDirection(): void {
    if (!this._overworld) {
      return;
    }
    if (this._pendingDirection !== null) {
      if (this._overworld.player_direction === undefined || this._overworld.player_direction === null) {
        this._overworld.player_direction = this._pendingDirection;
      }
      this._pendingDirection = null;
    }
  }

  get fixed_facing(): string | null {
    return this._fixed_facing;
  }

  set fixed_facing(value: string | null) {
    this._fixed_facing = value;
  }

  override get direction(): string {
    if (this._overworld) {
      return this._overworld.player_direction ?? this._pendingDirection ?? "down";
    }
    return this._pendingDirection ?? "down";
  }

  override set direction(value: string) {
    const normalized = String(value ?? "").toLowerCase();
    if (["up", "down", "left", "right"].includes(normalized)) {
      if (this._overworld) {
        this._overworld.player_direction = normalized;
      } else {
        this._pendingDirection = normalized;
      }
    }
  }

  override get x(): number {
    return this.overworld.player_x;
  }

  override set x(value: number) {
    this._setPlayerField("player_x", value);
  }

  override get y(): number {
    return this.overworld.player_y;
  }

  override set y(value: number) {
    this._setPlayerField("player_y", value);
  }

  override get prevX(): number {
    return this.overworld.prev_player_x;
  }

  override set prevX(value: number) {
    this._setPlayerField("prev_player_x", value);
  }

  override get prevY(): number {
    return this.overworld.prev_player_y;
  }

  override set prevY(value: number) {
    this._setPlayerField("prev_player_y", value);
  }

  override get collisionStride(): number {
    if (this._overworld) {
      return this._overworld.TILES_PER_COLLISION;
    }
    return this._collisionStride;
  }

  override set collisionStride(value: number) {
    this._collisionStride = value;
  }

  override get footprint(): number {
    return this.collisionStride - 1;
  }

  override set footprint(value: number) {
    this.collisionStride = value + 1;
  }

  override get pixelX(): number {
    return this.overworld.player_px_x;
  }

  override set pixelX(value: number) {
    this._setPlayerField("player_px_x", value);
  }

  // ASM parity: snake_case pixel access must reflect the live overworld player coords.
  override get pixel_x(): number {
    return this.pixelX;
  }

  override set pixel_x(value: number) {
    this.pixelX = value;
  }

  override get pixelY(): number {
    return this.overworld.player_px_y;
  }

  override set pixelY(value: number) {
    this._setPlayerField("player_px_y", value);
  }

  // ASM parity: snake_case pixel access must reflect the live overworld player coords.
  override get pixel_y(): number {
    return this.pixelY;
  }

  override set pixel_y(value: number) {
    this.pixelY = value;
  }

  override get targetPixelX(): number {
    return this.overworld.target_px_x;
  }

  override set targetPixelX(value: number) {
    this._setPlayerField("target_px_x", value);
  }

  override get target_pixel_x(): number {
    return this.targetPixelX;
  }

  override set target_pixel_x(value: number) {
    this.targetPixelX = value;
  }

  override get targetPixelY(): number {
    return this.overworld.target_px_y;
  }

  override set targetPixelY(value: number) {
    this._setPlayerField("target_px_y", value);
  }

  override get target_pixel_y(): number {
    return this.targetPixelY;
  }

  override set target_pixel_y(value: number) {
    this.targetPixelY = value;
  }

  override get stepDxPx(): number {
    return this.overworld.step_dx_px;
  }

  override set stepDxPx(value: number) {
    this._setPlayerField("step_dx_px", value);
  }

  override get stepDyPx(): number {
    return this.overworld.step_dy_px;
  }

  override set stepDyPx(value: number) {
    this._setPlayerField("step_dy_px", value);
  }

  public turn(direction: string, force: boolean = false): void {
    if (this._fixed_facing !== null && !force) {
      return;
    }
    const normalized = direction.toLowerCase();
    if (["up", "down", "left", "right"].includes(normalized)) {
      this.direction = normalized;
    }
  }

  public face_player(_player_x?: number | null, _player_y?: number | null): void {
    // Placeholder for parity with NPCs; the player proxy doesn't face itself.
  }

  public apply_movement(movement_data: Iterable<string>): void {
    for (const rawCommand of movement_data) {
      const parts = rawCommand.split(" ").filter(Boolean);
      if (!parts.length) {
        continue;
      }
      const command = parts[0].toLowerCase();
      if (command in PlayerCharacter.STEP_COMMANDS) {
        if (parts.length < 2) {
          continue;
        }
        this._move(parts[1].toLowerCase(), PlayerCharacter.STEP_COMMANDS[command]);
      } else if (command === "turn_head" && parts.length >= 2) {
        this.turn(parts[1]);
      } else if (command === "fix_facing") {
        this.fixed_facing = this.overworld.player_direction;
      } else if (command === "remove_fixed_facing") {
        this.fixed_facing = null;
      } else if (command === "set_sliding") {
        this.overworld.player_state = PlayerState.SKATE;
      } else if (command === "remove_sliding") {
        this.overworld.player_state = PlayerState.NORMAL;
      } else if (
        command === "tree_shake" ||
        command === "step_sleep" ||
        command === "teleport_from" ||
        command === "skyfall_top"
      ) {
        continue;
      } else if (command === "step_end") {
        break;
      }
    }
  }

  public update_pixel_position(): void {
    this.overworld._sync_player_state?.();
    this.targetPixelX = this.pixelX;
    this.targetPixelY = this.pixelY;
    this.stepDxPx = 0.0;
    this.stepDyPx = 0.0;
    this.step_frames_remaining = 0;
    this.step_total_frames = 0;
    this.walking = false;
    this.jumping = false;
    this.sprite_y_offset = 0.0;
  }

  private _move(direction: string, distance: number): void {
    const deltas: Record<string, [number, number]> = {
      left: [-1, 0],
      right: [1, 0],
      up: [0, -1],
      down: [0, 1],
    };
    const [dx, dy] = deltas[direction] ?? [0, 0];
    if (dx === 0 && dy === 0) {
      return;
    }
    const overworld = this.overworld;
    overworld.prev_player_x = overworld.player_x;
    overworld.prev_player_y = overworld.player_y;
    overworld.player_x += dx * distance;
    overworld.player_y += dy * distance;
    if (this._fixed_facing === null) {
      overworld.player_direction = direction;
    }
    overworld.stop_player_movement?.();
  }
}
