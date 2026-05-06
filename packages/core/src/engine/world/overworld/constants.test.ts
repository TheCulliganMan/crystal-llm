import { BADGE_ENGINE_FLAG_ORDER } from "@pokecrystal/core/core/badges";
import { resolveCollisionValue } from "@pokecrystal/core/engine/world/overworld/collision-data";
import { CollisionType, _BADGE_FLAG_NAMES } from "@pokecrystal/core/engine/world/overworld/constants";

describe("CollisionType values", () => {
  it("matches disassembly collision constants for overworld movement", () => {
    expect(CollisionType.CUT_TREE).toBe(resolveCollisionValue("CUT_TREE"));
    expect(CollisionType.CUT_TREE_ALT).toBe(resolveCollisionValue("CUT_TREE_1A"));
    expect(CollisionType.WHIRLPOOL).toBe(resolveCollisionValue("WHIRLPOOL"));
    expect(CollisionType.WHIRLPOOL_ALT).toBe(resolveCollisionValue("WHIRLPOOL_2C"));
    expect(CollisionType.WATERFALL).toBe(resolveCollisionValue("WATERFALL"));
    expect(CollisionType.WATERFALL_RIGHT).toBe(resolveCollisionValue("WATERFALL_RIGHT"));
    expect(CollisionType.WATERFALL_LEFT).toBe(resolveCollisionValue("WATERFALL_LEFT"));
    expect(CollisionType.WATERFALL_UP).toBe(resolveCollisionValue("WATERFALL_UP"));
    expect(CollisionType.CURRENT_DOWN).toBe(resolveCollisionValue("CURRENT_DOWN"));
    expect(CollisionType.TALL_GRASS).toBe(resolveCollisionValue("TALL_GRASS"));
    expect(CollisionType.LONG_GRASS).toBe(resolveCollisionValue("LONG_GRASS"));
    expect(CollisionType.LONG_GRASS_ALT).toBe(resolveCollisionValue("LONG_GRASS_1C"));
  });
});

describe("_BADGE_FLAG_NAMES", () => {
  it("matches ASM badge engine-flag ordering", () => {
    expect(Object.keys(_BADGE_FLAG_NAMES)).toHaveLength(BADGE_ENGINE_FLAG_ORDER.length);
    BADGE_ENGINE_FLAG_ORDER.forEach((flagName, index) => {
      expect(_BADGE_FLAG_NAMES[index]).toBe(flagName);
    });
  });
});
