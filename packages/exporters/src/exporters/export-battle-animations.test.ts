import fs from "fs";
import { exportBattleAnimations } from "./export-battle-animations";

const mockWriteJsonToTargets = jest.fn();

jest.mock("./asm-utils", () => {
  const actual = jest.requireActual("./asm-utils");
  return {
    ...actual,
    writeJsonToTargets: (...args: unknown[]) => mockWriteJsonToTargets(...args),
  };
});

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => "/mock/pokecrystal",
}));

describe("exportBattleAnimations", () => {
  beforeEach(() => {
    mockWriteJsonToTargets.mockReset();
    jest.restoreAllMocks();
  });

  it("splits battle animation labels and strips comments", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "BattleAnim_Tackle:",
      "\tanim_1gfx ANIM_GFX_HIT ; hit graphic",
      "\tanim_obj BATTLE_ANIM_OBJ_HIT_BIG_YFIX, 136, 56, $0",
      "",
      "BattleAnim_Empty:",
      "",
      "BattleAnim_Growl:",
      "\tanim_cry $0",
      "\tanim_ret ; done",
    ].join("\n"));

    const animations = exportBattleAnimations();

    expect(animations).toEqual({
      BattleAnim_Tackle: [
        "anim_1gfx ANIM_GFX_HIT",
        "anim_obj BATTLE_ANIM_OBJ_HIT_BIG_YFIX, 136, 56, $0",
      ],
      BattleAnim_Growl: ["anim_cry $0", "anim_ret"],
    });
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith("animations.json", animations);
  });

  it("preserves BattleAnim_Tackle-style opcode lines", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "BattleAnim_Tackle:",
      "\tanim_1gfx ANIM_GFX_HIT",
      "\tanim_sound 0, 1, SFX_TACKLE",
      "\tanim_obj BATTLE_ANIM_OBJ_HIT_BIG_YFIX, 136, 56, $0",
      "\tanim_wait 6",
      "\tanim_ret",
    ].join("\n"));

    expect(exportBattleAnimations().BattleAnim_Tackle).toEqual([
      "anim_1gfx ANIM_GFX_HIT",
      "anim_sound 0, 1, SFX_TACKLE",
      "anim_obj BATTLE_ANIM_OBJ_HIT_BIG_YFIX, 136, 56, $0",
      "anim_wait 6",
      "anim_ret",
    ]);
  });

  it("throws when no battle animation labels can be parsed", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue("NotBattleAnim:\n\tanim_ret\n");

    expect(() => exportBattleAnimations()).toThrow(
      "Could not parse battle animation scripts from /mock/pokecrystal/data/moves/animations.asm"
    );
  });
});
