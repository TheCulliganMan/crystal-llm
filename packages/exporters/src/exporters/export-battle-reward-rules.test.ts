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
      throw new Error(`Unexpected read ${filePath}`);
    });

    const { exportBattleRewardRules } = await import("./export-battle-reward-rules");

    expect(exportBattleRewardRules()).toEqual({
      max_level: 100,
      wild_exp_divisor: 7,
      trainer_exp_numerator: 3,
      trainer_exp_denominator: 2,
    });
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "battle_reward_rules.json",
      {
        max_level: 100,
        wild_exp_divisor: 7,
        trainer_exp_numerator: 3,
        trainer_exp_denominator: 2,
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
