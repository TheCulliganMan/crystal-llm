import fs from "fs";
import path from "path";
import type { Move } from "@pokecrystal/core/core/models/move";
import type { PokemonSpecies } from "@pokecrystal/core/core/models/pokemon";
import { GenderRatio, Stat } from "@pokecrystal/core/core/enums/pokemon";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { stripAsmComment, writeJsonToTargets } from "./asm-utils";

export type LevelUpLearnsets = Record<string, Array<[number, string]>>;
export type LevelUpMovesData = Record<string, Array<{ level: number; move: string }>>;
export type EggMovesData = Record<string, string[]>;
export type GrowthRateCurveData = {
  id: string;
  numerator: number;
  denominator: number;
  quadratic: number;
  linear: number;
  constant: number;
};

const STAT_MAPPING: Record<string, Stat> = {
  ATTACK: "ATTACK",
  DEFENSE: "DEFENSE",
  SPEED: "SPEED",
  SP_ATK: "SPECIAL_ATTACK",
  SP_DEF: "SPECIAL_DEFENSE",
  ACCURACY: "ACCURACY",
  EVASION: "EVASION",
};

export function parsePokemonConstants(constantsFilePath: string): Record<string, number> {
  const idMap: Record<string, number> = {};
  let idCounter = 0;
  for (const rawLine of fs.readFileSync(constantsFilePath, "utf8").split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    if (!line) continue;
    const parts = line.split(/\s+/);
    if (parts[0] === "const_def") {
      idCounter = parts[1] ? Number.parseInt(parts[1], 10) : 0;
      continue;
    }
    if (parts[0] === "const" && parts[1]) {
      idMap[parts[1]] = idCounter;
      idCounter += 1;
    }
  }
  return idMap;
}

function enumKeyOrThrow<T extends Record<string, unknown>>(mapping: T, key: string, label: string, filePath: string): keyof T {
  if (mapping[key as keyof T] === undefined) {
    throw new Error(`Unknown ${label} ${key} in ${filePath}`);
  }
  return key as keyof T;
}

export function parseBaseStats(filePath: string, idMap: Record<string, number>, weight = 0): PokemonSpecies {
  const content = fs.readFileSync(filePath, "utf8");
  const idMatch = content.match(/^\s*db\s+([A-Z_0-9]+)\s*;/m);
  if (!idMatch) throw new Error(`Could not find species ID in ${filePath}`);
  const speciesId = idMatch[1];
  const statsMatch = content.match(/db\s+(\d+),\s+(\d+),\s+(\d+),\s+(\d+),\s+(\d+),\s+(\d+)/);
  if (!statsMatch) throw new Error(`Could not find stats in ${filePath}`);
  const typeMatch = content.match(/db\s+([A-Z_]+),\s+([A-Z_]+)\s*;\s*type/);
  const catchRateMatch = content.match(/db\s+(\d+)\s*;\s*catch rate/);
  const baseExpMatch = content.match(/db\s+(\d+)\s*;\s*base exp/);
  const itemsMatch = content.match(/db\s+([A-Z_]+),\s+([A-Z_]+)\s*;\s*items/);
  const genderRatioMatch = content.match(/db\s+([A-Z0-9_]+)\s*;\s*gender ratio/);
  const stepCyclesMatch = content.match(/db\s+(\d+)\s*;\s*step cycles to hatch/);
  const growthRateMatch = content.match(/db\s+([A-Z_]+)\s*;\s*growth rate/);
  const eggGroupsMatch = content.match(/dn\s+([A-Z0-9_]+),\s+([A-Z0-9_]+)/);
  if (!typeMatch || !catchRateMatch || !baseExpMatch || !itemsMatch || !genderRatioMatch || !stepCyclesMatch || !growthRateMatch || !eggGroupsMatch) {
    throw new Error(`Could not fully parse base stats in ${filePath}`);
  }
  const tmhmMatch = content.match(/tmhm\s+([A-Z_ ,]+)/);
  const tmhmLearnset = tmhmMatch ? tmhmMatch[1].split(",").map((part) => part.trim()).filter(Boolean) : [];

  const type1 = typeMatch[1];
  const type2 = typeMatch[2];
  const genderRatio = enumKeyOrThrow(GenderRatio, genderRatioMatch[1], "gender ratio", filePath);
  const growthRate = growthRateMatch[1];
  const eggGroup1 = eggGroupsMatch[1];
  const eggGroup2 = eggGroupsMatch[2];
  const intId = idMap[speciesId];
  if (intId === undefined) {
    throw new Error(`Missing numeric species id for ${speciesId} in ${filePath}`);
  }

  return {
    id: speciesId,
    int_id: intId,
    base_stats: {
      hp: Number.parseInt(statsMatch[1], 10),
      attack: Number.parseInt(statsMatch[2], 10),
      defense: Number.parseInt(statsMatch[3], 10),
      speed: Number.parseInt(statsMatch[4], 10),
      special_attack: Number.parseInt(statsMatch[5], 10),
      special_defense: Number.parseInt(statsMatch[6], 10),
    },
    type1: type1 as PokemonSpecies["type1"],
    type2: type2 as PokemonSpecies["type2"],
    catch_rate: Number.parseInt(catchRateMatch[1], 10),
    base_exp: Number.parseInt(baseExpMatch[1], 10),
    item1: itemsMatch[1] === "NO_ITEM" ? null : itemsMatch[1],
    item2: itemsMatch[2] === "NO_ITEM" ? null : itemsMatch[2],
    gender_ratio: GenderRatio[genderRatio],
    unknown1: 0,
    step_cycles_to_hatch: Number.parseInt(stepCyclesMatch[1], 10),
    unknown2: 0,
    growth_rate: growthRate as PokemonSpecies["growth_rate"],
    egg_group1: eggGroup1 as PokemonSpecies["egg_group1"],
    egg_group2: eggGroup2 as PokemonSpecies["egg_group2"],
    tmhm_learnset: tmhmLearnset as PokemonSpecies["tmhm_learnset"],
    ability: "NONE",
    pic_size: 0,
    front_pic: 0,
    back_pic: 0,
    weight,
  } as PokemonSpecies;
}

export function parseGrowthRates(filePath: string): GrowthRateCurveData[] {
  const content = fs.readFileSync(filePath, "utf8");
  const rates: GrowthRateCurveData[] = [];
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripAsmComment(rawLine);
    const match = line.match(/^growth_rate\s+(-?\d+),\s*(-?\d+),\s*(-?\d+),\s*(-?\d+),\s*(-?\d+)/);
    if (!match) continue;
    const commentMatch = rawLine.match(/;\s*(.+)$/);
    if (!commentMatch) {
      throw new Error(`Missing growth-rate label comment in ${filePath}: ${rawLine}`);
    }
    const id = `GROWTH_${commentMatch[1].trim().replace(/\s+/g, "_").toUpperCase()}`;
    rates.push({
      id,
      numerator: Number.parseInt(match[1], 10),
      denominator: Number.parseInt(match[2], 10),
      quadratic: Number.parseInt(match[3], 10),
      linear: Number.parseInt(match[4], 10),
      constant: Number.parseInt(match[5], 10),
    });
  }
  if (rates.length === 0) {
    throw new Error(`No growth rates parsed from ${filePath}`);
  }
  return rates;
}

export function loadAllPokemonData(baseStatsPath: string, idMap: Record<string, number>): PokemonSpecies[] {
  const allPokemonData: PokemonSpecies[] = [];
  const dexEntriesDir = path.join(getDisassemblyRoot(), "data", "pokemon", "dex_entries");
  for (const [pokemonName, intId] of Object.entries(idMap).sort((a, b) => a[1] - b[1])) {
    if (pokemonName.startsWith("UNOWN_") || pokemonName === "EGG") {
      continue;
    }
    const filePath = path.join(baseStatsPath, `${pokemonName.toLowerCase()}.asm`);
    if (fs.existsSync(filePath)) {
      const dexEntryPath = path.join(dexEntriesDir, `${pokemonName.toLowerCase()}.asm`);
      if (!fs.existsSync(dexEntryPath)) {
        throw new Error(`Missing dex entry for ${pokemonName} at ${dexEntryPath}`);
      }
      const content = fs.readFileSync(dexEntryPath, "utf8");
      const weightMatch = content.match(/dw\s+\d+,\s*(\d+)\s*;\s*height, weight/);
      if (!weightMatch) {
        throw new Error(`Could not find dex weight for ${pokemonName} in ${dexEntryPath}`);
      }
      const weight = Number.parseInt(weightMatch[1], 10);
      allPokemonData.push(parseBaseStats(filePath, idMap, weight));
    }
  }
  return allPokemonData.sort((a, b) => a.int_id - b.int_id);
}

export function parseMoveEffectsFromContent(content: string): string[] {
  const effects: string[] = [];
  const seen = new Set<string>();
  for (const match of content.matchAll(/^\s*move\s+[A-Z0-9_]+,\s*([A-Z0-9_]+)/gm)) {
    const effectName = match[1].replace("EFFECT_", "");
    if (!seen.has(effectName)) {
      seen.add(effectName);
      effects.push(effectName);
    }
  }
  return effects;
}

export function generateMoveEffectEnumString(effects: string[]): string {
  return ["class MoveEffect(Enum):", '    NONE = "NONE"', ...effects.map((effect) => `    ${effect} = "${effect}"`)].join("\n");
}

export function updateEnumsFileContent(enumsContent: string, newEnumStr: string): string {
  return enumsContent.replace(/(class MoveEffect\(Enum\):.*?)(\n\nclass|\Z)/s, `${newEnumStr}$2`);
}

export function parseMoves(movesFilePath: string): Record<string, Move> {
  const movesMap: Record<string, Move> = {};
  const content = fs.readFileSync(movesFilePath, "utf8");
  for (const [lineIndex, rawLine] of content.split(/\r?\n/).entries()) {
    const line = rawLine.trim();
    if (!line.startsWith("move")) continue;
    const parts = line.split(",").map((part) => part.trim());
    if (parts.length < 7) {
      throw new Error(`Could not parse move row ${lineIndex + 1} in ${movesFilePath}: ${line}`);
    }
    const name = parts[0].replace("move ", "");
    let effect = parts[1].replace("EFFECT_", "");
    let stat: Move["stat"] = null;
    let amount: Move["amount"] = null;
    const statChangeMatch = effect.match(/^([A-Z_]+)_(UP|DOWN)_?(\d)?_?(HIT)?$/);
    if (statChangeMatch) {
      const [, statKey, direction, amountStr, isHit] = statChangeMatch;
      const statName = STAT_MAPPING[statKey];
      if (statName) {
        stat = statName;
        amount = amountStr ? Number.parseInt(amountStr, 10) : 1;
        if (direction === "DOWN" && amount !== null) amount *= -1;
        effect = `${statName}_${direction}${amountStr ?? ""}${isHit ? "_HIT" : ""}`;
      }
    }
    movesMap[name] = {
      name: name as Move["name"],
      type: parts[3] as Move["type"],
      power: Number.parseInt(parts[2], 10),
      accuracy: Number.parseInt(parts[4].replace(" percent", ""), 10),
      pp: Number.parseInt(parts[5], 10),
      effect: effect as Move["effect"],
      effect_chance: Number.parseInt(parts[6].replace(" percent", ""), 10),
      stat,
      amount,
    };
  }
  return movesMap;
}

function asmLabelForSpeciesId(speciesId: string): string {
  return speciesId
    .toLowerCase()
    .split("_")
    .map((part) => part.charAt(0).toUpperCase() + part.slice(1))
    .join("");
}

const buildSpeciesLabelMap = (idMap: Record<string, number>): Map<string, string> => {
  const labels = new Map<string, string>();
  for (const species of Object.keys(idMap)) {
    const label = asmLabelForSpeciesId(species);
    if (labels.has(label)) {
      throw new Error(`Duplicate ASM species label '${label}' from pokemon constants.`);
    }
    labels.set(label, species);
  }
  return labels;
};

const speciesFromEvosAttacksLabel = (
  label: string,
  speciesLabels: Map<string, string>
): string | null => {
  const rawSpecies = label.replace(/EvosAttacks$/, "");
  return speciesLabels.get(rawSpecies) ?? null;
};

const speciesFromEggMovesLabel = (label: string, speciesLabels: Map<string, string>): string | null => {
  const rawSpecies = label.replace(/EggMoves$/, "");
  return speciesLabels.get(rawSpecies) ?? null;
};

export function parseLearnsets(
  evosAttacksFilePath: string,
  idMap: Record<string, number>
): LevelUpLearnsets {
  const learnsets: LevelUpLearnsets = {};
  const speciesLabels = buildSpeciesLabelMap(idMap);
  const lines = fs.readFileSync(evosAttacksFilePath, "utf8").split(/\r?\n/);
  let currentSpecies: string | null = null;
  let readingLearnset = false;

  for (const rawLine of lines) {
    const line = stripAsmComment(rawLine);
    if (!line) {
      continue;
    }
    const labelMatch = line.match(/^([A-Za-z0-9_]+EvosAttacks):$/);
    if (labelMatch) {
      currentSpecies = speciesFromEvosAttacksLabel(labelMatch[1], speciesLabels);
      readingLearnset = false;
      if (!currentSpecies) {
        throw new Error(`Unknown or case-changed learnset species label '${labelMatch[1]}'.`);
      }
      learnsets[currentSpecies] = [];
      continue;
    }
    if (!currentSpecies || !line.startsWith("db")) {
      continue;
    }
    const payload = line.replace(/^db\s+/, "").trim();
    if (payload === "0") {
      if (readingLearnset) {
        currentSpecies = null;
        readingLearnset = false;
      } else {
        readingLearnset = true;
      }
      continue;
    }
    if (!readingLearnset) {
      continue;
    }
    const parts = payload.split(",").map((part) => part.trim()).filter(Boolean);
    if (parts.length < 2) {
      throw new Error(`Malformed level-up move row in ${evosAttacksFilePath}: ${line}`);
    }
    const level = Number.parseInt(parts[0], 10);
    const move = parts[1];
    if (!Number.isFinite(level) || !move) {
      throw new Error(`Malformed level-up move row in ${evosAttacksFilePath}: ${line}`);
    }
    learnsets[currentSpecies].push([level, move]);
  }

  return Object.fromEntries(
    Object.entries(learnsets).sort((a, b) => (idMap[a[0]] ?? 0) - (idMap[b[0]] ?? 0))
  );
}

export function buildLevelUpMovesData(learnsets: LevelUpLearnsets): LevelUpMovesData {
  return Object.fromEntries(
    Object.entries(learnsets).map(([species, moves]) => [
      species,
      moves.map(([level, move]) => ({ level, move })),
    ])
  );
}

export function parseEggMoves(eggMovesFilePath: string, idMap: Record<string, number>): EggMovesData {
  const eggMoves: EggMovesData = {};
  const speciesLabels = buildSpeciesLabelMap(idMap);
  const lines = fs.readFileSync(eggMovesFilePath, "utf8").split(/\r?\n/);
  let currentSpecies: string | null = null;

  for (const rawLine of lines) {
    const line = stripAsmComment(rawLine);
    if (!line || line.startsWith("INCLUDE") || line.startsWith("SECTION")) {
      continue;
    }
    const labelMatch = line.match(/^([A-Za-z0-9_]+EggMoves):$/);
    if (labelMatch) {
      if (labelMatch[1] === "NoEggMoves") {
        currentSpecies = null;
        continue;
      }
      currentSpecies = speciesFromEggMovesLabel(labelMatch[1], speciesLabels);
      if (!currentSpecies) {
        throw new Error(`Unknown or case-changed egg-move species label '${labelMatch[1]}'.`);
      }
      eggMoves[currentSpecies] = [];
      continue;
    }
    if (!currentSpecies || !line.startsWith("db")) {
      continue;
    }
    const move = line.replace(/^db\s+/, "").trim();
    if (move === "-1") {
      currentSpecies = null;
      continue;
    }
    eggMoves[currentSpecies].push(move);
  }

  return Object.fromEntries(
    Object.entries(eggMoves).sort((a, b) => (idMap[a[0]] ?? 0) - (idMap[b[0]] ?? 0))
  );
}

export function exportData(): {
  pokemonData: PokemonSpecies[];
  movesData: Record<string, Move>;
  learnsetsData: LevelUpLearnsets;
  levelUpMovesData: LevelUpMovesData;
  eggMovesData: EggMovesData;
  growthRatesData: GrowthRateCurveData[];
} {
  const root = getDisassemblyRoot();
  const constantsPath = path.join(root, "constants", "pokemon_constants.asm");
  const baseStatsPath = path.join(root, "data", "pokemon", "base_stats");
  const movesPath = path.join(root, "data", "moves", "moves.asm");
  const learnsetsPath = path.join(root, "data", "pokemon", "evos_attacks.asm");
  const eggMovesPath = path.join(root, "data", "pokemon", "egg_moves.asm");
  const growthRatesPath = path.join(root, "data", "growth_rates.asm");
  const idMap = parsePokemonConstants(constantsPath);
  const pokemonData = loadAllPokemonData(baseStatsPath, idMap);
  const movesData = parseMoves(movesPath);
  const learnsetsData = parseLearnsets(learnsetsPath, idMap);
  const levelUpMovesData = buildLevelUpMovesData(learnsetsData);
  const eggMovesData = parseEggMoves(eggMovesPath, idMap);
  const growthRatesData = parseGrowthRates(growthRatesPath);
  writeJsonToTargets("pokemon_data.json", pokemonData, { indent: 2 });
  writeJsonToTargets("moves_data.json", movesData, { indent: 2 });
  writeJsonToTargets("learnsets.json", learnsetsData, { indent: 2 });
  writeJsonToTargets("level_up_moves.json", levelUpMovesData, { indent: 2 });
  writeJsonToTargets("egg_moves.json", eggMovesData, { indent: 2 });
  writeJsonToTargets("growth_rates.json", growthRatesData, { indent: 2 });
  return { pokemonData, movesData, learnsetsData, levelUpMovesData, eggMovesData, growthRatesData };
}
