const mockReadFileSync = jest.fn();
const mockWriteJsonToTargets = jest.fn();

jest.mock("fs", () => ({
  readFileSync: (...args: unknown[]) => mockReadFileSync(...args),
}));

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => "/mock/pokecrystal",
}));

jest.mock("./asm-utils", () => ({
  parseAsmNumber: (value: string) => {
    if (value.startsWith("$")) return Number.parseInt(value.slice(1), 16);
    if (value.startsWith("%")) return Number.parseInt(value.slice(1), 2);
    return Number.parseInt(value, 10);
  },
  writeJsonToTargets: (...args: unknown[]) => mockWriteJsonToTargets(...args),
}));

const asmByPath = (filePath: string): string => {
  if (filePath.endsWith("engine/overworld/events.asm")) {
    return `
.skip_happiness:
  ld a, [wStepCount]
  cp $80
  jr nz, .skip_egg
.skip_egg:
  ld hl, wPoisonStepCount
  ld a, [hl]
  cp 4
  jr c, .skip_poison
`;
  }
  if (filePath.endsWith("engine/pokemon/breeding.asm")) {
    return `
DoEggStep::
  cp EGG
  dec [hl]
  jr nz, .next
HatchEggs:
  cp EGG
  ld a, [hl]
  and a
  jp nz, .next
  ld [hl], $78
`;
  }
  if (filePath.endsWith("engine/events/happiness_egg.asm")) {
    return `
StepHappiness::
  ld hl, wHappinessStepCount
  ld a, [hl]
  inc a
  and 1
  ld [hl], a
  ret nz
`;
  }
  if (filePath.endsWith("engine/events/poisonstep.asm")) {
    return `
.DamageMonIfPoisoned:
  and 1 << PSN
  dec bc
  ld [hl], 0
  ld c, %10
  ld c, %01
`;
  }
  if (filePath.endsWith("engine/events/daycare.asm")) {
    return `
.String_EGG:
  db "EGG@"
`;
  }
  throw new Error(`Unexpected read ${filePath}`);
};

describe("export-step-event-rules", () => {
  beforeEach(() => {
    jest.resetModules();
    mockReadFileSync.mockReset();
    mockWriteJsonToTargets.mockReset();
    mockReadFileSync.mockImplementation(asmByPath);
  });

  it("exports step event rules from ASM", async () => {
    const { exportStepEventRules } = await import("./export-step-event-rules");

    expect(exportStepEventRules()).toEqual({
      poison_step_interval: 4,
      egg_step_trigger: 0x80,
      hatched_egg_happiness: 0x78,
      poison_status: "POISON",
      egg_nickname: "EGG",
      happiness_step_counter_mask: 1,
      happiness_step_counter_target: 0,
    });
    expect(mockWriteJsonToTargets).toHaveBeenCalledWith(
      "step_event_rules.json",
      {
        poison_step_interval: 4,
        egg_step_trigger: 0x80,
        hatched_egg_happiness: 0x78,
        poison_status: "POISON",
        egg_nickname: "EGG",
        happiness_step_counter_mask: 1,
        happiness_step_counter_target: 0,
      },
      { indent: 2 }
    );
  });

  it("rejects missing poison damage pattern", async () => {
    mockReadFileSync.mockImplementation((filePath: string) => {
      if (filePath.endsWith("engine/events/poisonstep.asm")) {
        return ".DamageMonIfPoisoned:\n  ret\n";
      }
      return asmByPath(filePath);
    });
    const { exportStepEventRules } = await import("./export-step-event-rules");

    expect(() => exportStepEventRules()).toThrow("DoPoisonStep does not contain expected instruction");
  });
});

export {};
