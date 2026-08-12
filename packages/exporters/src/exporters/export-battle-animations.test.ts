import fs from "fs";
import path from "path";
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
  const fallthroughTargets = [
    "BattleAnimSub_Return:",
    "\tanim_bgeffect BATTLE_BG_EFFECT_RETURN_MON, $0, BG_EFFECT_USER, $0",
    "\tanim_ret",
    "BattleAnim_MirrorMove:",
    "\tanim_ret",
    "BattleAnim_Sonicboom:",
    "\tanim_2gfx BATTLE_ANIM_GFX_WIND, BATTLE_ANIM_GFX_HIT",
    "\tanim_ret",
    "BattleAnim_StunSpore:",
    "\tanim_1gfx BATTLE_ANIM_GFX_POWDER",
    "\tanim_ret",
  ];

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
      ...fallthroughTargets,
    ].join("\n"));

    const animations = exportBattleAnimations();

    expect(animations).toEqual({
      BattleAnim_Tackle: [
        "anim_1gfx ANIM_GFX_HIT",
        "anim_obj BATTLE_ANIM_OBJ_HIT_BIG_YFIX, 136, 56, $0",
      ],
      BattleAnim_Growl: ["anim_cry $0", "anim_ret"],
      BattleAnimSub_Return: [
        "anim_bgeffect BATTLE_BG_EFFECT_RETURN_MON, $0, BG_EFFECT_USER, $0",
        "anim_ret",
      ],
      BattleAnim_ReturnMon: [
        "anim_bgeffect BATTLE_BG_EFFECT_RETURN_MON, $0, BG_EFFECT_USER, $0",
        "anim_ret",
      ],
      BattleAnim_Dummy: ["anim_ret"],
      BattleAnim_Gust: ["anim_2gfx BATTLE_ANIM_GFX_WIND, BATTLE_ANIM_GFX_HIT", "anim_ret"],
      BattleAnim_MirrorMove: ["anim_ret"],
      BattleAnim_Poisonpowder: ["anim_1gfx BATTLE_ANIM_GFX_POWDER", "anim_ret"],
      BattleAnim_SleepPowder: ["anim_1gfx BATTLE_ANIM_GFX_POWDER", "anim_ret"],
      BattleAnim_Sonicboom: ["anim_2gfx BATTLE_ANIM_GFX_WIND, BATTLE_ANIM_GFX_HIT", "anim_ret"],
      BattleAnim_Spore: ["anim_1gfx BATTLE_ANIM_GFX_POWDER", "anim_ret"],
      BattleAnim_StunSpore: ["anim_1gfx BATTLE_ANIM_GFX_POWDER", "anim_ret"],
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
      ...fallthroughTargets,
    ].join("\n"));

    expect(exportBattleAnimations().BattleAnim_Tackle).toEqual([
      "anim_1gfx ANIM_GFX_HIT",
      "anim_sound 0, 1, SFX_TACKLE",
      "anim_obj BATTLE_ANIM_OBJ_HIT_BIG_YFIX, 136, 56, $0",
      "anim_wait 6",
      "anim_ret",
    ]);
  });

  it("exports shared BattleAnimSub sections for anim_call resolution", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "BattleAnim_ReturnMon:",
      "\tanim_sound 0, 0, SFX_BALL_POOF",
      "BattleAnim_Absorb:",
      "\tanim_call BattleAnimSub_Drain",
      "\tanim_ret",
      "BattleAnimSub_Drain:",
      "\tanim_obj BATTLE_ANIM_OBJ_DRAIN, 132, 44, $0",
      "\tanim_ret",
      ...fallthroughTargets,
    ].join("\n"));

    const animations = exportBattleAnimations();

    expect(animations.BattleAnim_Absorb).toEqual([
      "anim_call BattleAnimSub_Drain",
      "anim_ret",
    ]);
    expect(animations.BattleAnimSub_Drain).toEqual([
      "anim_obj BATTLE_ANIM_OBJ_DRAIN, 132, 44, $0",
      "anim_ret",
    ]);
    expect(animations.BattleAnim_ReturnMon).toEqual([
      "anim_sound 0, 0, SFX_BALL_POOF",
      "anim_bgeffect BATTLE_BG_EFFECT_RETURN_MON, $0, BG_EFFECT_USER, $0",
      "anim_ret",
    ]);
  });

  it("normalizes real ASM local-label declarations for Rust branch resolution", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "BattleAnim_ThrowPokeBall:",
      "\tanim_if_param_equal MASTER_BALL, .MasterBall",
      ".MasterBall:",
      "\tanim_jump .Shake",
      ".Shake:",
      "\tanim_ret",
      ...fallthroughTargets,
    ].join("\n"));

    expect(exportBattleAnimations().BattleAnim_ThrowPokeBall).toEqual([
      "anim_if_param_equal MASTER_BALL, .MasterBall",
      ".MasterBall",
      "anim_jump .Shake",
      ".Shake",
      "anim_ret",
    ]);
  });

  it("throws when no battle animation labels can be parsed", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue("NotBattleAnim:\n\tanim_ret\n");

    expect(() => exportBattleAnimations()).toThrow(
      "Could not parse battle animation scripts from /mock/pokecrystal/data/moves/animations.asm"
    );
  });

  it("rejects unresolved local and shared ASM control-flow targets", () => {
    jest.spyOn(fs, "readFileSync").mockReturnValue([
      "BattleAnim_Tackle:",
      "\tanim_call BattleAnimSub_Missing",
      ...fallthroughTargets,
    ].join("\n"));

    expect(() => exportBattleAnimations()).toThrow(
      "Battle animation 'BattleAnim_Tackle' command 0 references missing target 'BattleAnimSub_Missing'",
    );
  });

  it("exports the complete canonical ASM control-flow graph", () => {
    const source = fs.readFileSync(
      path.resolve(__dirname, "../../../../vendor/pokecrystal/data/moves/animations.asm"),
      "utf8",
    );
    jest.spyOn(fs, "readFileSync").mockReturnValue(source);

    const animations = exportBattleAnimations();
    const subroutines = Object.keys(animations).filter((label) => label.startsWith("BattleAnimSub_"));

    expect(Object.keys(animations)).toHaveLength(298);
    expect(subroutines).toHaveLength(16);
    expect(animations.BattleAnim_ThrowPokeBall).toContain(".TheTrainerBlockedTheBall");
    expect(animations.BattleAnim_ThrowPokeBall).not.toContain(".TheTrainerBlockedTheBall:");
    expect(animations.BattleAnimSub_Drain[0]).toBe(
      "anim_obj BATTLE_ANIM_OBJ_DRAIN, 132, 44, $0",
    );
    expect(animations.BattleAnim_ReturnMon).toEqual([
      "anim_sound 0, 0, SFX_BALL_POOF",
      "anim_bgeffect BATTLE_BG_EFFECT_RETURN_MON, $0, BG_EFFECT_USER, $0",
      "anim_wait 32",
      "anim_ret",
    ]);
  });
});
