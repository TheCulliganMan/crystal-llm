import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

export type ExportedBattleEscapeRules = {
  player_speed_multiplier: number;
  enemy_speed_divisor: number;
  failed_attempt_bonus: number;
  rng_roll_values: number;
};

export function exportBattleEscapeRules(): ExportedBattleEscapeRules {
  const corePath = path.join(getDisassemblyRoot(), "engine", "battle", "core.asm");
  const lines = fs.readFileSync(corePath, "utf8").split(/\r?\n/).map((line) => stripAsmComment(line));
  const start = lines.findIndex((line) => line === "TryToRunAwayFromBattle:");
  const end = lines.findIndex((line, index) => index > start && line === ".cant_escape");
  if (start < 0 || end < 0) {
    throw new Error("Could not locate TryToRunAwayFromBattle escape formula");
  }
  const body = lines.slice(start, end);
  const hasEnemySpeedDivisor =
    body.filter((line) => line === "srl b").length >= 2 && body.filter((line) => line === "rr a").length >= 2;
  if (!hasEnemySpeedDivisor) {
    throw new Error("Could not verify battle escape enemy speed divisor");
  }
  if (!body.includes("ld a, 32")) {
    throw new Error("Could not verify battle escape player speed multiplier");
  }
  if (!body.includes("ld b, 30")) {
    throw new Error("Could not verify battle escape failed attempt bonus");
  }
  if (!body.includes("call BattleRandom")) {
    throw new Error("Could not verify battle escape 8-bit random roll");
  }
  const payload: ExportedBattleEscapeRules = {
    player_speed_multiplier: 32,
    enemy_speed_divisor: 4,
    failed_attempt_bonus: 30,
    rng_roll_values: 256,
  };
  writeJsonToTargets("battle_escape_rules/rules.json", payload, { indent: 2 });
  return payload;
}
