import fs from "fs";
import path from "path";
import type { PokemonSpecies } from "@pokecrystal/core/core/models/pokemon";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

export type ExportedCaptureBallRule = {
  multiplier_numerator: number;
  multiplier_denominator: number;
  battle_type: string;
  skip_hp_calc: boolean;
  use_heavy_ball_weight_modifier: boolean;
  use_level_ball_multiplier: boolean;
  require_same_species: boolean;
  require_same_gender: boolean;
  require_fast_species: boolean;
};

export type ExportedCaptureRules = {
  fast_ball_species: string[];
  heavy_ball_modifiers: Record<string, number>;
  ball_rules: Record<string, ExportedCaptureBallRule>;
  guaranteed_capture_balls: string[];
  status_bonus: Record<string, number>;
};

const ballRule = (
  multiplierNumerator = 1,
  multiplierDenominator = 1,
  options: {
    battleType?: string;
    skipHpCalc?: boolean;
    useHeavyBallWeightModifier?: boolean;
    useLevelBallMultiplier?: boolean;
    requireSameSpecies?: boolean;
    requireSameGender?: boolean;
    requireFastSpecies?: boolean;
  } = {}
): ExportedCaptureBallRule => ({
  multiplier_numerator: multiplierNumerator,
  multiplier_denominator: multiplierDenominator,
  battle_type: options.battleType ?? "",
  skip_hp_calc: options.skipHpCalc ?? false,
  use_heavy_ball_weight_modifier: options.useHeavyBallWeightModifier ?? false,
  use_level_ball_multiplier: options.useLevelBallMultiplier ?? false,
  require_same_species: options.requireSameSpecies ?? false,
  require_same_gender: options.requireSameGender ?? false,
  require_fast_species: options.requireFastSpecies ?? false,
});

const expectedBallTable = [
  "ULTRA_BALL",
  "GREAT_BALL",
  "SAFARI_BALL",
  "HEAVY_BALL",
  "LEVEL_BALL",
  "LURE_BALL",
  "FAST_BALL",
  "MOON_BALL",
  "LOVE_BALL",
  "PARK_BALL",
];

const parseBallMultiplierTable = (): string[] => {
  const filePath = path.join(getDisassemblyRoot(), "engine", "items", "item_effects.asm");
  const lines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
  const rows: string[] = [];
  let inTable = false;
  for (const raw of lines) {
    const line = stripAsmComment(raw);
    if (line === "BallMultiplierFunctionTable:") {
      inTable = true;
      continue;
    }
    if (!inTable || !line) {
      continue;
    }
    if (line === "db -1") {
      break;
    }
    const row = line.match(/^dbw\s+([A-Z0-9_]+),\s*([A-Za-z0-9_]+)$/);
    if (!row) {
      throw new Error(`Malformed BallMultiplierFunctionTable row: ${raw}`);
    }
    rows.push(row[1]);
  }
  if (rows.join("|") !== expectedBallTable.join("|")) {
    throw new Error(`Unexpected BallMultiplierFunctionTable order: ${rows.join(", ")}`);
  }
  return rows;
};

const heavyBallModifier = (weight: number): number => {
  if (weight < 1024) return -20;
  if (weight < 2048) return 0;
  if (weight < 3072) return 20;
  if (weight < 4096) return 30;
  return 40;
};

export function exportCaptureRules(pokemonData: PokemonSpecies[]): ExportedCaptureRules {
  parseBallMultiplierTable();
  const heavyBallModifiers = Object.fromEntries(
    pokemonData.map((pokemon) => [pokemon.id, heavyBallModifier(pokemon.weight)])
  );
  const payload: ExportedCaptureRules = {
    fast_ball_species: ["MAGNEMITE", "GRIMER", "TANGELA"],
    heavy_ball_modifiers: heavyBallModifiers,
    ball_rules: {
      MASTER_BALL: ballRule(),
      POKE_BALL: ballRule(),
      FRIEND_BALL: ballRule(),
      ULTRA_BALL: ballRule(2, 1),
      GREAT_BALL: ballRule(3, 2),
      SAFARI_BALL: ballRule(3, 2),
      PARK_BALL: ballRule(3, 2),
      HEAVY_BALL: ballRule(1, 1, { useHeavyBallWeightModifier: true }),
      LEVEL_BALL: ballRule(1, 1, { skipHpCalc: true, useLevelBallMultiplier: true }),
      LURE_BALL: ballRule(3, 1, { battleType: "BATTLETYPE_FISH" }),
      MOON_BALL: ballRule(),
      LOVE_BALL: ballRule(8, 1, { requireSameSpecies: true, requireSameGender: true }),
      FAST_BALL: ballRule(4, 1, { requireFastSpecies: true }),
    },
    guaranteed_capture_balls: ["MASTER_BALL"],
    status_bonus: {
      SLEEP: 10,
      FREEZE: 10,
    },
  };
  writeJsonToTargets("capture_rules/rules.json", payload, { indent: 2 });
  return payload;
}
