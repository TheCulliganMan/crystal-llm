import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

export type ExportedBattleRewardRules = {
  max_level: number;
  wild_exp_divisor: number;
  trainer_exp_numerator: number;
  trainer_exp_denominator: number;
  mom_money_increment: number;
  mom_random_items: ExportedMomPurchase[];
  mom_progression_items: ExportedMomPurchase[];
};

export type ExportedMomPurchase = {
  trigger: number;
  cost: number;
  kind: "item" | "doll";
  target: string;
  decoration_flag: string | null;
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

const parseMomMoneyIncrement = (content: string): number => {
  for (const rawLine of content.split(/\r?\n/)) {
    const match = stripAsmComment(rawLine).match(/^DEF\s+MOM_MONEY\s+EQU\s+(\d+)$/);
    if (match) {
      return Number(match[1]);
    }
  }
  throw new Error("Unable to find DEF MOM_MONEY EQU in misc constants.");
};

const parseDecorationFlags = (content: string): Set<string> => {
  const flags = new Set<string>();
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const match = line.match(/^decoration\s+[^,]+\s*,\s*[^,]+\s*,\s*[^,]+\s*,\s*(EVENT_[A-Z0-9_]+)\s*,/);
    if (match) {
      flags.add(match[1]);
    }
  }
  if (flags.size === 0) {
    throw new Error("Unable to find decoration event flags in decoration attributes.");
  }
  return flags;
};

const parseMomItemSet = (
  content: string,
  label: "MomItems_1" | "MomItems_2",
  decorationFlags: ReadonlySet<string>
): ExportedMomPurchase[] => {
  const lines = content.split(/\r?\n/);
  const start = lines.findIndex((line) => stripAsmComment(line) === `${label}:`);
  if (start < 0) {
    throw new Error(`Unable to find ${label} in Mom item data.`);
  }
  const purchases: ExportedMomPurchase[] = [];
  for (const rawLine of lines.slice(start + 1)) {
    const line = stripAsmComment(rawLine);
    if (line === ".End") {
      return purchases;
    }
    if (!line) {
      continue;
    }
    const match = line.match(/^momitem\s+(\d+)\s*,\s*(\d+)\s*,\s*(MOM_ITEM|MOM_DOLL)\s*,\s*([A-Z0-9_]+)$/);
    if (!match) {
      throw new Error(`Malformed ${label} row: ${line}`);
    }
    const kind = match[3] === "MOM_DOLL" ? "doll" : "item";
    const target = match[4];
    const expectedDecorationFlag = `EVENT_${target}`;
    const decorationFlag = kind === "doll" && decorationFlags.has(expectedDecorationFlag)
      ? expectedDecorationFlag
      : null;
    if (kind === "doll" && !decorationFlag) {
      throw new Error(`${label} doll ${target} has no decoration event flag`);
    }
    purchases.push({
      trigger: Number(match[1]),
      cost: Number(match[2]),
      kind,
      target,
      decoration_flag: decorationFlag ?? null,
    });
  }
  throw new Error(`${label} has no .End marker.`);
};

export function exportBattleRewardRules(): ExportedBattleRewardRules {
  const battleConstants = readAsm("constants/battle_constants.asm");
  const battleCore = readAsm("engine/battle/core.asm");
  assertTrainerExpBoostPattern(battleCore);
  const miscConstants = readAsm("constants/misc_constants.asm");
  const momItems = readAsm("data/items/mom_phone.asm");
  const decorationFlags = parseDecorationFlags(readAsm("data/decorations/attributes.asm"));
  const payload: ExportedBattleRewardRules = {
    max_level: parseMaxLevel(battleConstants),
    wild_exp_divisor: parseWildExpDivisor(battleCore),
    trainer_exp_numerator: 3,
    trainer_exp_denominator: 2,
    mom_money_increment: parseMomMoneyIncrement(miscConstants),
    mom_random_items: parseMomItemSet(momItems, "MomItems_1", decorationFlags),
    mom_progression_items: parseMomItemSet(momItems, "MomItems_2", decorationFlags),
  };
  writeJsonToTargets("battle_reward_rules.json", payload, { indent: 2 });
  writeJsonToTargets(
    path.join("content-packs", "core-modular", "battle_reward_rules", "rules.json"),
    payload,
    { indent: 2 },
  );
  return payload;
}
