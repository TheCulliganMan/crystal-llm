import fs from "fs";
import os from "os";
import path from "path";
import { exportBattleEscapeRules } from "./export-battle-escape-rules";

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

describe("exportBattleEscapeRules", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-battle-escape-rules-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockAssetsRoot = path.join(tempDir, "assets");
    writeFile(
      path.join(mockDisassemblyRoot, "engine", "battle", "core.asm"),
      [
        "TryToRunAwayFromBattle:",
        "\tld a, 32",
        "\tldh [hMultiplier], a",
        "\tsrl b",
        "\trr a",
        "\tsrl b",
        "\trr a",
        "\tld b, 30",
        "\tcall BattleRandom",
        ".cant_escape",
        "",
      ].join("\n")
    );
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("exports battle escape formula constants from exact ASM pattern", () => {
    const rules = exportBattleEscapeRules();

    expect(rules).toEqual({
      player_speed_multiplier: 32,
      enemy_speed_divisor: 4,
      failed_attempt_bonus: 30,
      rng_roll_values: 256,
    });
    expect(JSON.parse(fs.readFileSync(path.join(mockAssetsRoot, "data", "battle_escape_rules", "rules.json"), "utf8"))).toEqual(
      rules
    );
  });
});
