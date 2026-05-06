import type { OverworldObject } from "../overworld-object";
import { ScriptTask } from "./script-task";
import type { OverworldContext } from "@pokecrystal/core/engine/world/story-events/commands/base";

type FollowParticipant = OverworldObject & {
  prev_x?: number;
  prev_y?: number;
  collisionStride?: number;
  collision_stride?: number;
  update_pixel_position?: () => void;
};

type FollowOverworldContext = OverworldContext & {
  WALK_FRAMES?: number;
  player_direction?: string;
  TILES_PER_COLLISION?: number;
  _sync_player_state?: () => void;
};

export class FollowTask extends ScriptTask<FollowOverworldContext> {
  private readonly follower: FollowParticipant | null;
  private readonly leader: FollowParticipant | null;
  private framesRemaining = 0;

  constructor(
    follower: FollowParticipant | null,
    leader: FollowParticipant | null,
    options: { onComplete?: (() => void) | null } = {}
  ) {
    super({ blocking: true, onComplete: options.onComplete ?? null });
    this.follower = follower;
    this.leader = leader;
  }

  start(overworld: FollowOverworldContext): void {
    super.start(overworld);
    this.framesRemaining = Math.max(1, Math.trunc(overworld.WALK_FRAMES ?? 1));
  }

  update(overworld: FollowOverworldContext): void {
    if (this.completed) {
      return;
    }
    if (this.framesRemaining > 0) {
      this.framesRemaining -= 1;
      if (this.framesRemaining === 0) {
        this.syncDirection(overworld);
        this.completed = true;
      }
    }
  }

  private syncDirection(overworld: FollowOverworldContext): void {
    if (!this.follower || !this.leader) {
      return;
    }
    let leaderDirection: string | null = null;
    if (this.leader?.name === "PLAYER") {
      leaderDirection = overworld.player_direction ?? null;
    } else {
      leaderDirection = this.leader?.direction ?? null;
    }

    if (leaderDirection) {
      this.applyDirection(this.follower, leaderDirection, overworld);
    }

    this.alignFollower(overworld);
  }

  private applyDirection(follower: FollowParticipant, direction: string, overworld: FollowOverworldContext): void {
    if (typeof follower.turn === "function") {
      try {
        follower.turn(direction);
        return;
      } catch {
        // Fall back to assignment.
      }
    }
    if ("direction" in follower) {
      follower.direction = direction;
    }
    if (follower.name === "PLAYER") {
      overworld.player_direction = direction;
    }
  }

  private alignFollower(overworld: FollowOverworldContext): void {
    const follower = this.follower;
    const leader = this.leader;
    if (!follower || !leader) {
      return;
    }

    const followerX = follower.x;
    const followerY = follower.y;
    const leaderX = leader.x;
    const leaderY = leader.y;
    if ([followerX, followerY, leaderX, leaderY].some((value) => value === null || value === undefined)) {
      return;
    }

    const stride = follower.collision_stride ?? follower.collisionStride ?? overworld.TILES_PER_COLLISION ?? 1;
    const dx = leaderX - followerX;
    const dy = leaderY - followerY;

    if (Math.abs(dx) >= stride) {
      const step = dx > 0 ? stride : -stride;
      this.applyStep(follower, step, 0, overworld);
    } else if (Math.abs(dy) >= stride) {
      const step = dy > 0 ? stride : -stride;
      this.applyStep(follower, 0, step, overworld);
    }
  }

  private applyStep(
    follower: FollowParticipant,
    deltaX: number,
    deltaY: number,
    overworld: FollowOverworldContext
  ): void {
    let direction: string | null = null;
    if (deltaX > 0) {
      direction = "right";
    } else if (deltaX < 0) {
      direction = "left";
    } else if (deltaY > 0) {
      direction = "down";
    } else if (deltaY < 0) {
      direction = "up";
    }

    if (deltaX) {
      const currentX = follower.x;
      if ("prev_x" in follower) {
        follower.prev_x = currentX;
      }
      follower.x = currentX + deltaX;
    }
    if (deltaY) {
      const currentY = follower.y;
      if ("prev_y" in follower) {
        follower.prev_y = currentY;
      }
      follower.y = currentY + deltaY;
    }

    if (direction) {
      this.applyDirection(follower, direction, overworld);
    }

    if (typeof follower.update_pixel_position === "function") {
      follower.update_pixel_position();
    } else if (follower.name === "PLAYER") {
      overworld._sync_player_state?.();
    }
    // ASM: CopyCoordsTileToLastCoordsTile after follower movement completes.
    follower.prev_x = follower.x;
    follower.prev_y = follower.y;
    follower.prevX = follower.x;
    follower.prevY = follower.y;
  }
}
