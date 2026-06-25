import fs from "fs";
import os from "os";
import path from "path";
import { exportCaptureRules } from "./export-capture-rules";

let mockDisassemblyRoot = "";
let mockAssetsRoot = "";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot,
  getAssetsRoot: () => mockAssetsRoot,
}));

const writeFile = (filePath: string, content: string): void => {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, content);
};

describe("exportCaptureRules", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-capture-rules-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "items", "item_effects.asm"),
      [
        "BallMultiplierFunctionTable:",
        "\tdbw ULTRA_BALL,  UltraBallMultiplier",
        "\tdbw GREAT_BALL,  GreatBallMultiplier",
        "\tdbw SAFARI_BALL, SafariBallMultiplier",
        "\tdbw HEAVY_BALL,  HeavyBallMultiplier",
        "\tdbw LEVEL_BALL,  LevelBallMultiplier",
        "\tdbw LURE_BALL,   LureBallMultiplier",
        "\tdbw FAST_BALL,   FastBallMultiplier",
        "\tdbw MOON_BALL,   MoonBallMultiplier",
        "\tdbw LOVE_BALL,   LoveBallMultiplier",
        "\tdbw PARK_BALL,   ParkBallMultiplier",
        "\tdb -1 ; end",
        "",
      ].join("\n")
    );
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports exact capture ball behavior and explicit Heavy Ball modifiers", () => {
    const rules = exportCaptureRules([
      { id: "LIGHTMON", int_id: 1, weight: 1000 },
      { id: "MIDMON", int_id: 2, weight: 2500 },
      { id: "HEAVYMON", int_id: 3, weight: 5000 },
    ] as any);

    expect(rules.ball_rules.ULTRA_BALL).toMatchObject({
      multiplier_numerator: 2,
      multiplier_denominator: 1,
    });
    expect(rules.ball_rules.LEVEL_BALL).toMatchObject({
      use_level_ball_multiplier: true,
      skip_hp_calc: true,
    });
    expect(rules.ball_rules.LURE_BALL).toMatchObject({
      multiplier_numerator: 3,
      battle_type: "BATTLETYPE_FISH",
    });
    expect(rules.ball_rules.MOON_BALL).toMatchObject({
      multiplier_numerator: 1,
      multiplier_denominator: 1,
      use_heavy_ball_weight_modifier: false,
      use_level_ball_multiplier: false,
      require_same_species: false,
      require_same_gender: false,
      require_fast_species: false,
    });
    expect(rules.ball_rules.HEAVY_BALL.use_heavy_ball_weight_modifier).toBe(true);
    expect(rules.ball_rules.LOVE_BALL).toMatchObject({
      require_same_species: true,
      require_same_gender: true,
      multiplier_numerator: 8,
    });
    expect(rules.ball_rules.FAST_BALL).toMatchObject({
      require_fast_species: true,
      multiplier_numerator: 4,
    });
    expect(rules.guaranteed_capture_balls).toEqual(["MASTER_BALL"]);
    expect(rules.status_bonus).toEqual({ SLEEP: 10, FREEZE: 10 });
    expect(rules.heavy_ball_modifiers).toEqual({
      LIGHTMON: -20,
      MIDMON: 20,
      HEAVYMON: 40,
    });
    expect(JSON.parse(fs.readFileSync(path.join(mockAssetsRoot, "data", "capture_rules", "rules.json"), "utf8"))).toEqual(
      rules
    );
  });
});
