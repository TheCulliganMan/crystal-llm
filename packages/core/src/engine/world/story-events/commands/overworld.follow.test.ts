import { createInitialGameState } from "@pokecrystal/core/core/state";
import { EventManager } from "@pokecrystal/core/engine/world/events";
import type { OverworldEngine } from "@pokecrystal/core/engine/world/overworld/overworld";
import { FollowCommand } from "./overworld";

describe("FollowCommand", () => {
  test("starts following immediately without pausing scripts (ASM Script_follow)", () => {
    const gameState = createInitialGameState();
    const eventManager = new EventManager(gameState);
    const leader = { object_id: "LEADER" };
    const follower = { object_id: "FOLLOWER" };
    const start_following = jest.fn();
    const overworld = {
      get_object_by_id: (id: string) => (id === "LEADER" ? leader : id === "FOLLOWER" ? follower : null),
      start_following,
    } as unknown as OverworldEngine;
    const runner = { pause: jest.fn(), resume: jest.fn(), last_condition_result: null };
    const command = new FollowCommand("LEADER", "FOLLOWER");
    command.runner = runner as unknown as typeof command.runner;

    command.execute(gameState, eventManager, overworld);

    expect(start_following).toHaveBeenCalledWith(follower, leader, {
      follower_id: "FOLLOWER",
      leader_id: "LEADER",
      followerId: "FOLLOWER",
      leaderId: "LEADER",
    });
    expect(runner.pause).not.toHaveBeenCalled();
    expect(runner.resume).not.toHaveBeenCalled();
    expect(runner.last_condition_result).toBe(true);
  });
});
