import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

export type ExportedBattleRewardRules = {
  max_level: number;
  wild_exp_divisor: number;
  trainer_exp_numerator: number;
  trainer_exp_denominator: number;
};

const readAsm = (relativePath: string): string =>
  fs.readFileSync(path.join(getDisassemblyRoot(), relativePath), "utf8");

const parseMaxLevel = (content: string): number => {
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const match = line.match(/^DEF\s+MAX_LEVEL\s+EQU\s+(\d+)$/);
    if (match) {
      return Number(match[1]);
    }
  }
  throw new Error("Unable to find DEF MAX_LEVEL EQU in battle constants.");
};

const parseWildExpDivisor = (content: string): number => {
  const label = content.indexOf("GiveExperiencePoints:");
  if (label < 0) {
    throw new Error("Unable to find GiveExperiencePoints label.");
  }
  const slice = content.slice(label, label + 2200);
  const match = slice.match(/ld\s+a,\s*\[wEnemyMonBaseExp\][\s\S]*?\n\s*ld\s+a,\s*(\d+)\s*\n\s*ldh\s+\[hDivisor\],\s*a/);
  if (!match) {
    throw new Error("Unable to find wild experience divisor in GiveExperiencePoints.");
  }
  return Number(match[1]);
};

const assertTrainerExpBoostPattern = (content: string): void => {
  const label = content.indexOf("BoostExp:");
  if (label < 0) {
    throw new Error("Unable to find BoostExp label.");
  }
  const slice = content.slice(label, label + 800);
  for (const expected of ["srl b", "rr c", "add c", "adc b"]) {
    if (!slice.includes(expected)) {
      throw new Error(`BoostExp does not contain expected 1.5x instruction '${expected}'.`);
    }
  }
};

export function exportBattleRewardRules(): ExportedBattleRewardRules {
  const battleConstants = readAsm("constants/battle_constants.asm");
  const battleCore = readAsm("engine/battle/core.asm");
  const payload: ExportedBattleRewardRules = {
    max_level: parseMaxLevel(battleConstants),
    wild_exp_divisor: parseWildExpDivisor(battleCore),
    trainer_exp_numerator: 3,
    trainer_exp_denominator: 2,
  };
  assertTrainerExpBoostPattern(battleCore);
  writeJsonToTargets("battle_reward_rules.json", payload, { indent: 2 });
  return payload;
}
