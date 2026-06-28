import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { writeJsonToTargets } from "./asm-utils";

export type ExportedFishingSlot = {
  threshold: number;
  species: string | null;
  level: number;
  time_group: string | null;
};

export type ExportedRodTable = {
  slots: ExportedFishingSlot[];
};

export type ExportedFishingGroup = {
  bite_threshold: number;
  rod_tables: Record<string, ExportedRodTable>;
};

export type ExportedTimeFishEntry = {
  day_species: string;
  day_level: number;
  night_species: string;
  night_level: number;
};

export type ExportedFishingSwarmRule = {
  daily_flag_bit: number;
  swarm: number;
  base_group: string;
  swarm_group: string;
};

export type ExportedFishingCatalog = {
  groups: Record<string, ExportedFishingGroup>;
  time_groups: Record<string, ExportedTimeFishEntry>;
  swarm_rules: Record<string, ExportedFishingSwarmRule>;
  rod_items: Record<string, string>;
};

const ROD_IDS = ["OLD_ROD", "GOOD_ROD", "SUPER_ROD"] as const;

const stripComment = (line: string): string => line.replace(/;.*/, "").trim();

const parseThreshold = (raw: string): number => {
  const match = raw.trim().match(/^(\d+)\s+percent(?:\s*\+\s*1)?$/);
  if (!match) {
    throw new Error(`Unsupported fishing threshold expression '${raw}'.`);
  }
  const percent = Number(match[1]);
  const base = Math.floor((Math.min(percent, 100) * 0xff) / 100);
  return raw.includes("+") ? Math.min(base + 1, 255) : base;
};

const parseFishGroupConstants = (content: string): string[] => {
  const constants: string[] = [];
  let inFishGroups = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (line === "const_def") {
      inFishGroups = true;
      continue;
    }
    if (!inFishGroups) continue;
    const match = line.match(/^const\s+(FISHGROUP_[A-Z0-9_]+)$/);
    if (!match) {
      if (constants.length > 0 && line.startsWith("DEF ")) break;
      continue;
    }
    if (match[1] !== "FISHGROUP_NONE") {
      constants.push(match[1]);
    }
  }
  if (constants.length === 0) {
    throw new Error("No FISHGROUP_* constants found.");
  }
  return constants;
};

type RawGroupRow = {
  biteThreshold: number;
  labels: string[];
};

const parseFishGroupRows = (content: string): RawGroupRow[] => {
  const rows: RawGroupRow[] = [];
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    const match = line.match(
      /^fishgroup\s+(.+?),\s*(\.[A-Za-z0-9_]+),\s*(\.[A-Za-z0-9_]+),\s*(\.[A-Za-z0-9_]+)$/,
    );
    if (!match) continue;
    rows.push({
      biteThreshold: parseThreshold(match[1]),
      labels: [match[2], match[3], match[4]].map((label) => label.slice(1)),
    });
  }
  if (rows.length === 0) {
    throw new Error("No fishgroup rows found.");
  }
  return rows;
};

const parseRodTables = (content: string): Map<string, ExportedRodTable> => {
  const labels = new Map<string, ExportedRodTable>();
  let activeLabels: string[] = [];
  let slots: ExportedFishingSlot[] = [];

  const flush = (): void => {
    if (activeLabels.length === 0) return;
    const table = { slots };
    for (const label of activeLabels) {
      labels.set(label, table);
    }
    activeLabels = [];
    slots = [];
  };

  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (line === "TimeFishGroups:") {
      flush();
      break;
    }
    const label = line.match(/^\.(\w+):$/);
    if (label) {
      if (slots.length > 0) {
        flush();
      }
      activeLabels.push(label[1]);
      continue;
    }
    if (activeLabels.length === 0) continue;
    const timeMatch = line.match(/^db\s+(.+?),\s*time_group\s+(\d+)$/);
    if (timeMatch) {
      slots.push({
        threshold: parseThreshold(timeMatch[1]),
        species: null,
        level: 0,
        time_group: `TIME_GROUP_${Number(timeMatch[2])}`,
      });
      continue;
    }
    const speciesMatch = line.match(/^db\s+(.+?),\s*([A-Z0-9_]+),\s*(\d+)$/);
    if (speciesMatch) {
      slots.push({
        threshold: parseThreshold(speciesMatch[1]),
        species: speciesMatch[2],
        level: Number(speciesMatch[3]),
        time_group: null,
      });
      continue;
    }
  }
  return labels;
};

const parseTimeFishGroups = (
  content: string,
): Record<string, ExportedTimeFishEntry> => {
  const entries: Record<string, ExportedTimeFishEntry> = {};
  let inSection = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (line === "TimeFishGroups:") {
      inSection = true;
      continue;
    }
    if (!inSection || line.length === 0) continue;
    const match = line.match(
      /^db\s+([A-Z0-9_]+),\s*(\d+),\s*([A-Z0-9_]+),\s*(\d+)$/,
    );
    if (!match) {
      throw new Error(`Unsupported TimeFishGroups row '${line}'.`);
    }
    entries[`TIME_GROUP_${Object.keys(entries).length}`] = {
      day_species: match[1],
      day_level: Number(match[2]),
      night_species: match[3],
      night_level: Number(match[4]),
    };
  }
  if (Object.keys(entries).length === 0) {
    throw new Error("No TimeFishGroups entries found.");
  }
  return entries;
};

const parseConstDefValues = (
  content: string,
  prefix: string,
): Map<string, number> => {
  const values = new Map<string, number>();
  let current = 0;
  let inConstDef = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (line === "const_def") {
      current = 0;
      inConstDef = true;
      continue;
    }
    if (!inConstDef) continue;
    const match = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (match) {
      if (match[1].startsWith(prefix)) {
        values.set(match[1], current);
      }
      current += 1;
      continue;
    }
    if (line.startsWith("DEF ") && values.size > 0) {
      break;
    }
  }
  if (values.size === 0) {
    throw new Error(`No constants with prefix ${prefix} found.`);
  }
  return values;
};

const parseDailyFlagBit = (content: string): number => {
  let current = 0;
  let inDailyFlags = false;
  for (const rawLine of content.split(/\r?\n/)) {
    if (rawLine.trim() === "; wDailyFlags1::") {
      inDailyFlags = true;
      continue;
    }
    const line = stripComment(rawLine);
    if (!inDailyFlags) continue;
    if (line === "const_def") {
      current = 0;
      continue;
    }
    const match = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (match) {
      if (match[1] === "DAILYFLAGS1_FISH_SWARM_F") {
        return current;
      }
      current += 1;
      continue;
    }
    if (line.startsWith("; ") && current > 0) {
      break;
    }
  }
  throw new Error(
    "Could not parse DAILYFLAGS1_FISH_SWARM_F from ram_constants.asm.",
  );
};

const parseFishingSwarmRules = (
  root: string,
): Record<string, ExportedFishingSwarmRule> => {
  const fishEngine = fs
    .readFileSync(path.join(root, "engine", "events", "fish.asm"), "utf8")
    .split(/\r?\n/)
    .map(stripComment);
  const scriptConstants = fs.readFileSync(
    path.join(root, "constants", "script_constants.asm"),
    "utf8",
  );
  const ramConstants = fs.readFileSync(
    path.join(root, "constants", "ram_constants.asm"),
    "utf8",
  );
  const swarmValues = parseConstDefValues(scriptConstants, "FISHSWARM_");
  const dailyFlagBit = parseDailyFlagBit(ramConstants);
  const branchBaseGroups = new Map<string, string>();
  for (let index = 0; index < fishEngine.length - 1; index += 1) {
    const cpMatch = fishEngine[index].match(/^cp\s+(FISHGROUP_[A-Z0-9_]+)$/);
    const branchMatch = fishEngine[index + 1].match(
      /^jr z,\s*\.([A-Za-z0-9_]+)$/,
    );
    if (cpMatch && branchMatch) {
      branchBaseGroups.set(branchMatch[1], cpMatch[1]);
    }
  }
  const rules: Record<string, ExportedFishingSwarmRule> = {};
  for (const [label, baseGroup] of branchBaseGroups) {
    const labelIndex = fishEngine.indexOf(`.${label}`);
    if (labelIndex < 0) {
      throw new Error(`Fishing swarm branch .${label} is missing.`);
    }
    const block = fishEngine.slice(labelIndex + 1, labelIndex + 8);
    const swarmConstant = block
      .map((line) => line.match(/^cp\s+(FISHSWARM_[A-Z0-9_]+)$/)?.[1])
      .find(Boolean);
    const targetGroup = block
      .map((line) => line.match(/^ld d,\s*(FISHGROUP_[A-Z0-9_]+)$/)?.[1])
      .find(Boolean);
    if (!swarmConstant || !targetGroup) {
      throw new Error(
        `Fishing swarm branch .${label} does not declare an exact swarm and group.`,
      );
    }
    const swarm = swarmValues.get(swarmConstant);
    if (swarm === undefined) {
      throw new Error(
        `Fishing swarm branch .${label} references unknown ${swarmConstant}.`,
      );
    }
    rules[`SWARM_RULE_${Object.keys(rules).length}`] = {
      daily_flag_bit: dailyFlagBit,
      swarm,
      base_group: baseGroup,
      swarm_group: targetGroup,
    };
  }
  if (Object.keys(rules).length === 0) {
    throw new Error(
      "No fishing swarm rules were exported from engine/events/fish.asm.",
    );
  }
  return rules;
};

const parseFishingRodItemRules = (root: string): Record<string, string> => {
  const rawLines = fs
    .readFileSync(
      path.join(root, "engine", "items", "item_effects.asm"),
      "utf8",
    )
    .split(/\r?\n/);
  const lines = rawLines.map(stripComment);
  const itemLabels = new Map<string, string>();
  for (const rawLine of rawLines) {
    const match = rawLine
      .trim()
      .match(/^dw\s+([A-Za-z0-9_]+)\s+;\s+([A-Z0-9_]+)$/);
    if (match) {
      if (itemLabels.has(match[2])) {
        throw new Error(
          `Fishing rod item ${match[2]} is declared more than once.`,
        );
      }
      itemLabels.set(match[2], match[1]);
    }
  }
  const labelRods = new Map<string, string>();
  for (let index = 0; index < lines.length; index += 1) {
    const label = lines[index].match(/^([A-Za-z0-9_]+):$/)?.[1];
    if (!label) continue;
    const block = lines.slice(index + 1, index + 5);
    const rodIndex = block
      .map((line) => line.match(/^ld e,\s*\$(\d+)$/)?.[1])
      .find(Boolean);
    const jumpsToUseRod = block.some((line) => line === "jr UseRod");
    if (rodIndex === undefined || !jumpsToUseRod) continue;
    const rod = ROD_IDS[Number(rodIndex)];
    if (!rod) {
      throw new Error(
        `Fishing rod effect ${label} loads unsupported rod index ${rodIndex}.`,
      );
    }
    labelRods.set(label, rod);
  }
  const rules: Record<string, string> = {};
  for (const [itemId, label] of itemLabels) {
    const rod = labelRods.get(label);
    if (rod) {
      rules[itemId] = rod;
    }
  }
  if (Object.keys(rules).length !== ROD_IDS.length) {
    throw new Error(
      `Expected ${ROD_IDS.length} fishing rod item rules, exported ${Object.keys(rules).length}.`,
    );
  }
  return rules;
};

export function exportFishing(): ExportedFishingCatalog {
  const root = getDisassemblyRoot();
  const constants = parseFishGroupConstants(
    fs.readFileSync(
      path.join(root, "constants", "map_data_constants.asm"),
      "utf8",
    ),
  );
  const fishContent = fs.readFileSync(
    path.join(root, "data", "wild", "fish.asm"),
    "utf8",
  );
  const groupRows = parseFishGroupRows(fishContent);
  if (constants.length !== groupRows.length) {
    throw new Error(
      `Fishing group constant count ${constants.length} does not match fish table rows ${groupRows.length}.`,
    );
  }
  const rodTables = parseRodTables(fishContent);
  const groups: Record<string, ExportedFishingGroup> = {};
  for (let index = 0; index < constants.length; index += 1) {
    const row = groupRows[index];
    const group: ExportedFishingGroup = {
      bite_threshold: row.biteThreshold,
      rod_tables: {},
    };
    for (let rodIndex = 0; rodIndex < ROD_IDS.length; rodIndex += 1) {
      const label = row.labels[rodIndex];
      const table = rodTables.get(label);
      if (!table) {
        throw new Error(
          `Fishing group ${constants[index]} references missing table ${label}.`,
        );
      }
      group.rod_tables[ROD_IDS[rodIndex]] = table;
    }
    groups[constants[index]] = group;
  }
  const catalog = {
    groups,
    time_groups: parseTimeFishGroups(fishContent),
    swarm_rules: parseFishingSwarmRules(root),
    rod_items: parseFishingRodItemRules(root),
  };
  writeJsonToTargets("fishing.json", catalog, { indent: 2 });
  return catalog;
}
