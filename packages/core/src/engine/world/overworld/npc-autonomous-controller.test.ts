import { createInitialGameState } from "@pokecrystal/core/core/state";
import type { ObjectEvent } from "@pokecrystal/core/core/models/map";
import type { HardwareRNG } from "@pokecrystal/core/engine/games/rng";
import { NpcAutonomousController } from "./npc-autonomous-controller";
import { OverworldObject } from "./overworld-object";

class StubRng {
  private readonly addValue: number;

  constructor(addValue: number) {
    this.addValue = addValue;
  }

  nextByte(): number {
    return 0;
  }

  peekHRandomAdd(): number {
    return this.addValue;
  }

  randrange(_size: number): number {
    return 0;
  }
}

const buildTrainerEvent = (movement: string): ObjectEvent =>
  ({
    sprite: "SPRITE_YOUNGSTER",
    x: 0,
    y: 0,
    spritemovedata: movement,
    move_range_x: 0,
    move_range_y: 0,
    hram_x: 0,
    hram_y: 0,
    pal: 0,
    object_type: "OBJECTTYPE_TRAINER",
    radius: 3,
    script: "TrainerSightlineScript",
    event_flag: "",
    object_identifier: null,
    sightline_direction_override: null,
  }) as unknown as ObjectEvent;

describe("NpcAutonomousController spin cadence", () => {
  it("rotates clockwise trainers every sixteen frames like ASM spin movement", () => {
    const npc = new OverworldObject(buildTrainerEvent("SPRITEMOVEDATA_SPINCLOCKWISE"));
    npc.direction = "right";

    const overworld = {
      game_state: createInitialGameState(),
      npcs: [npc],
      TILES_PER_COLLISION: 2,
    } as unknown as ConstructorParameters<typeof NpcAutonomousController>[0];

    const controller = new NpcAutonomousController(overworld, {
      rng_factory: () => new StubRng(0x7f) as unknown as HardwareRNG,
    });

    controller.rebuild([npc]);

    for (let frame = 0; frame < 15; frame += 1) {
      controller.update();
      expect(npc.direction).toBe("right");
    }

    controller.update();
    expect(npc.direction).toBe("down");

    for (let frame = 0; frame < 16; frame += 1) {
      controller.update();
    }
    expect(npc.direction).toBe("left");
  });
});

describe("NpcAutonomousController collision", () => {
  it("keeps zero-range wander NPCs contained on their spawn subtile", () => {
    const npc = new OverworldObject(buildTrainerEvent("SPRITEMOVEDATA_WANDER"));
    npc.x = 10;
    npc.y = 10;
    npc.initialSubtileX = 10;
    npc.initialSubtileY = 10;

    const overworld = {
      game_state: createInitialGameState(),
      npcs: [npc],
      TILES_PER_COLLISION: 2,
      WALK_FRAMES: 1,
      _npc_step_blocked: jest.fn(() => false),
    } as unknown as ConstructorParameters<typeof NpcAutonomousController>[0];

    const controller = new NpcAutonomousController(overworld, {
      rng_factory: () => new StubRng(0) as unknown as HardwareRNG,
    });

    controller.rebuild([npc]);
    for (let frame = 0; frame < 8; frame += 1) {
      controller.update();
    }

    expect(npc.x).toBe(10);
    expect(npc.y).toBe(10);
    expect(overworld._npc_step_blocked).not.toHaveBeenCalled();
  });

  it("keeps a resumed script-paused wander NPC inside its configured movement range", () => {
    const npc = new OverworldObject({
      ...buildTrainerEvent("SPRITEMOVEDATA_WANDER"),
      move_range_x: 1,
      move_range_y: 1,
    });
    npc.x = 10;
    npc.y = 10;
    npc.initialSubtileX = 10;
    npc.initialSubtileY = 10;

    const gameState = createInitialGameState();
    const overworld = {
      game_state: gameState,
      npcs: [npc],
      TILES_PER_COLLISION: 2,
      WALK_FRAMES: 1,
      _npc_step_blocked: jest.fn(() => false),
      script_runner: { _script_stack: [{}], _awaiting_resume: 0 },
    } as unknown as ConstructorParameters<typeof NpcAutonomousController>[0] & {
      script_runner: { _script_stack: unknown[]; _awaiting_resume: number };
    };

    const controller = new NpcAutonomousController(overworld, {
      rng_factory: () => new StubRng(0) as unknown as HardwareRNG,
    });
    controller.rebuild([npc]);
    controller.update();

    npc.x = 12;
    npc.y = 10;
    overworld.script_runner._script_stack = [];
    controller.update();

    expect(npc.x).toBe(12);
    expect(npc.y).toBeGreaterThanOrEqual(8);
    expect(npc.y).toBeLessThanOrEqual(12);
    expect(overworld._npc_step_blocked).not.toHaveBeenCalledWith(npc, "right", 14, 10);
  });

  it("does not advance an autonomous NPC when the step-start probe catches a blocked sign tile", () => {
    const npc = new OverworldObject({
      ...buildTrainerEvent("SPRITEMOVEDATA_WALK_UP_DOWN"),
      move_range_y: 1,
    });
    npc.x = 0;
    npc.y = 0;
    npc.prevX = 0;
    npc.prevY = 0;

    const stepBlocked = jest
      .fn()
      .mockReturnValueOnce(false)
      .mockReturnValueOnce(true);

    const overworld = {
      game_state: createInitialGameState(),
      npcs: [npc],
      TILES_PER_COLLISION: 2,
      WALK_FRAMES: 1,
      _npc_step_blocked: stepBlocked,
    } as unknown as ConstructorParameters<typeof NpcAutonomousController>[0];

    const controller = new NpcAutonomousController(overworld, {
      rng_factory: () => new StubRng(0) as unknown as HardwareRNG,
    });

    controller.rebuild([npc]);
    controller.update();

    expect(stepBlocked).toHaveBeenCalledTimes(2);
    expect(stepBlocked).toHaveBeenNthCalledWith(1, npc, "up", 0, -2);
    expect(stepBlocked).toHaveBeenNthCalledWith(2, npc, "up", 0, -2, {
      suppress_blocked_log: true,
    });
    expect(npc.x).toBe(0);
    expect(npc.y).toBe(0);
    expect(npc.walking).toBe(false);
  });
});
