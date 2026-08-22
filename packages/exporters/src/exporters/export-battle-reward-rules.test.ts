const mockReadFileSync = jest.fn();
const mockWriteJsonToTargets = jest.fn();

jest.mock("fs", () => ({
  readFileSync: (...args: unknown[]) => mockReadFileSync(...args),
}));

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => "/mock/pokecrystal",
}));

jest.mock("./asm-utils", () => ({
  stripAsmComment: (line: string) => line.replace(/;.*/, "").trim(),
  writeJsonToTargets: (...args: unknown[]) => mockWriteJsonToTargets(...args),
}));

describe("export-battle-reward-rules", () => {
  beforeEach(() => {
    jest.resetModules();
    mockReadFileSync.mockReset();
    mockWriteJsonToTargets.mockReset();
  });

  it("exports level and experience rules from ASM", async () => {
    mockReadFileSync.mockImplementation((filePath: string) => {
      if (filePath.endsWith("constants/battle_constants.asm")) {
        return "DEF MAX_LEVEL EQU 100\n";
      }
      if (filePath.endsWith("engine/battle/core.asm")) {
        return `
GiveExperiencePoints:
  ld a, [wEnemyMonBaseExp]
  ld [hMultiplicand + 2], a
  ld a, 7
  ldh [hDivisor], a

BoostExp:
  ld b, h
  ld c, l
  srl b
  rr c
  add c
  adc b
`;
      }
      if (filePath.endsWith("constants/misc_constants.asm")) {
        return "DEF MOM_MONEY EQU 2300\n";
      }
      if (filePath.endsWith("data/items/mom_phone.asm")) {
        return `
MomItems_1:
  momitem 0, 600, MOM_ITEM, SUPER_POTION
.End
MomItems_2:
  momitem 900, 600, MOM_ITEM, SUPER_POTION
  momitem 10000, 1800, MOM_DOLL, DECO_CHARMANDER_DOLL
.End
`;
      }
      if (filePath.endsWith("data/decorations/attributes.asm")) {
        return "decoration DECO_DOLL, CHARMANDER, SET_UP_DOLL, EVENT_DECO_CHARMANDER_DOLL, SPRITE_CHARMANDER\n";
      }
      throw new Error(`Unexpected read ${filePath}`);
    });

    const { exportBattleRewardRules } = await import("./export-battle-reward-rules");

    expect(exportBattleRewardRules()).toEqual({
      max_level: 100,
      wild_exp_divisor: 7,
      trainer_exp_numerator: 3,
      trainer_exp_denominator: 2,
      mom_money_increment: 2300,
      mom_random_items: [
        { trigger: 0, cost: 600, kind: "item", target: "SUPER_POTION", decoration_flag: null },
      ],
      mom_progression_items: [
        { trigger: 900, cost: 600, kind: "item", target: "SUPER_POTION", decoration_flag: null },
        {
          trigger: 10000,
          cost: 1800,
          kind: "doll",
          target: "DECO_CHARMANDER_DOLL",
          decoration_flag: "EVENT_DECO_CHARMANDER_DOLL",
        },
      ],
    });
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "battle_reward_rules.json",
      {
        max_level: 100,
        wild_exp_divisor: 7,
        trainer_exp_numerator: 3,
        trainer_exp_denominator: 2,
        mom_money_increment: 2300,
        mom_random_items: [
          { trigger: 0, cost: 600, kind: "item", target: "SUPER_POTION", decoration_flag: null },
        ],
        mom_progression_items: [
          { trigger: 900, cost: 600, kind: "item", target: "SUPER_POTION", decoration_flag: null },
          {
            trigger: 10000,
            cost: 1800,
            kind: "doll",
            target: "DECO_CHARMANDER_DOLL",
            decoration_flag: "EVENT_DECO_CHARMANDER_DOLL",
          },
        ],
      },
      { indent: 2 }
    );
  });

  it("rejects missing trainer boost instructions", async () => {
    mockReadFileSync.mockImplementation((filePath: string) => {
      if (filePath.endsWith("constants/battle_constants.asm")) {
        return "DEF MAX_LEVEL EQU 100\n";
      }
      if (filePath.endsWith("engine/battle/core.asm")) {
        return `
GiveExperiencePoints:
  ld a, [wEnemyMonBaseExp]
  ld a, 7
  ldh [hDivisor], a
BoostExp:
  ret
`;
      }
      throw new Error(`Unexpected read ${filePath}`);
    });

    const { exportBattleRewardRules } = await import("./export-battle-reward-rules");

    expect(() => exportBattleRewardRules()).toThrow("BoostExp does not contain expected 1.5x instruction");
  });
});

export {};
