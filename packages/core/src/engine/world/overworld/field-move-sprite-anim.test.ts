import { createInitialGameState } from "@pokecrystal/core/core/state";
import {
  CutAnimationState,
  FieldMoveSpriteManager,
  FlyAnimationState,
  SPRITE_ANIM_OBJ_FLY_LEAF,
  SPRITE_ANIM_OBJ_CUT_TREE,
  SPRITE_ANIM_OBJ_LEAF,
} from "./field-move-sprite-anim";

describe("CutAnimationState", () => {
  it("spawns only the tree sprite for the tree variant", () => {
    const state = createInitialGameState();
    const manager = new FieldMoveSpriteManager();
    manager.reserve_oam(state);
    const cut = new CutAnimationState(manager, state, {
      player_x: 0,
      player_y: 0,
      target_tile_x: 3,
      target_tile_y: 4,
      direction: "down",
      variant: "tree",
    });

    cut.tick();

    const structs = manager.get_structs();
    expect(structs).toHaveLength(1);
    expect(structs[0].frameset_id).toBe(SPRITE_ANIM_OBJ_CUT_TREE);
    expect(cut.jumptable_index).toBe(2);
    expect(cut.frame_counter).toBe(32);
  });

  it("spawns leaf sprites for the grass variant", () => {
    const state = createInitialGameState();
    const manager = new FieldMoveSpriteManager();
    manager.reserve_oam(state);
    const cut = new CutAnimationState(manager, state, {
      player_x: 0,
      player_y: 0,
      target_tile_x: 3,
      target_tile_y: 4,
      direction: "down",
      variant: "grass",
    });

    cut.tick();

    const structs = manager.get_structs();
    expect(structs).toHaveLength(4);
    structs.forEach((struct) => {
      expect(struct.frameset_id).toBe(SPRITE_ANIM_OBJ_LEAF);
    });
    expect(cut.jumptable_index).toBe(2);
    expect(cut.frame_counter).toBe(32);
  });
});

describe("FlyAnimationState", () => {
  it("spawns the first leaf on the first timer tick like FlyFunction_FrameTimer", () => {
    const state = createInitialGameState();
    const manager = new FieldMoveSpriteManager();
    manager.reserve_oam(state, { base_addr: 0 });
    const fly = new FlyAnimationState(manager, state, {
      player_x: 80,
      player_y: 80,
      variant: "from",
    });

    fly.tick();

    const leaves = manager
      .get_structs()
      .filter((struct) => struct.frameset_id === SPRITE_ANIM_OBJ_FLY_LEAF);
    expect(leaves).toHaveLength(1);
    expect(leaves[0].y_coord).toBe(80 + 0x40);
    expect(state.wram.wFrameCounter).toBe(127);
    expect(state.wram.wFrameCounter2).toBe(1);
  });

  it("plays fly SFX from frame 128 through frame 64 using the pre-decrement counter", () => {
    const state = createInitialGameState();
    const manager = new FieldMoveSpriteManager();
    const soundPlayer = jest.fn();
    manager.reserve_oam(state, { base_addr: 0 });
    const fly = new FlyAnimationState(manager, state, {
      player_x: 80,
      player_y: 80,
      variant: "from",
      sound_player: soundPlayer,
    });

    for (let frame = 0; frame < 65; frame += 1) {
      fly.tick();
    }

    expect(soundPlayer.mock.calls.map(([sound]) => sound)).toEqual(Array(9).fill("SFX_FLY"));
    expect(state.wram.wFrameCounter).toBe(63);
  });

  it("sets the exit flag one tick after the frame counter reaches zero", () => {
    const state = createInitialGameState();
    const manager = new FieldMoveSpriteManager();
    manager.reserve_oam(state, { base_addr: 0 });
    const fly = new FlyAnimationState(manager, state, {
      player_x: 80,
      player_y: 80,
      variant: "to",
    });

    for (let frame = 0; frame < 64; frame += 1) {
      fly.tick();
    }

    expect(fly.completed).toBe(false);
    expect(state.wram.wFrameCounter).toBe(0);

    fly.tick();

    expect(fly.completed).toBe(true);
    expect(state.wram.wJumptableIndex & 0x80).toBe(0x80);
  });
});
