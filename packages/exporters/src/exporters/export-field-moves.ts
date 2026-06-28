import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { writeJsonToTargets } from "./asm-utils";

export type ExportedFieldMoveBadgeRequirement = {
  region: "johto";
  index: number;
};

export type ExportedFieldMoveRule = {
  move_id: string;
  badge: ExportedFieldMoveBadgeRequirement;
};

export type ExportedFieldMoveMoveRule = {
  move_id: string;
};

export type ExportedFieldEscapeItemRule = {
  item_id: string;
  escape_rope_mode: string;
};

export type ExportedFieldRepelItemRule = Record<string, never>;

export type ExportedFieldItemRule = {
  item_id: string;
};

export type ExportedFieldMoveReplacement = {
  replacement_block_id: number;
  variant: string;
};

export type ExportedFieldMoveBlockRule = {
  move_id: string;
  badge: ExportedFieldMoveBadgeRequirement;
  target_collisions: number[];
  replacements: Record<string, Record<string, ExportedFieldMoveReplacement>>;
};

export type ExportedFieldMoveFlagRule = {
  move_id: string;
  badge: ExportedFieldMoveBadgeRequirement;
  engine_flag: string;
};

export type ExportedFieldMoveTravelRule = {
  move_id: string;
  badge: ExportedFieldMoveBadgeRequirement;
  blocked_collisions: number[];
  target_collisions: number[];
};

export type ExportedFieldMoveCatalog = {
  cut: ExportedFieldMoveBlockRule;
  whirlpool: ExportedFieldMoveBlockRule;
  strength: ExportedFieldMoveFlagRule;
  flash: ExportedFieldMoveFlagRule;
  surf: ExportedFieldMoveTravelRule;
  waterfall: ExportedFieldMoveTravelRule;
  fly: ExportedFieldMoveRule;
  dig: ExportedFieldMoveMoveRule;
  teleport: ExportedFieldMoveMoveRule;
  escape_rope: ExportedFieldEscapeItemRule;
  repel: ExportedFieldRepelItemRule;
  bicycle: ExportedFieldItemRule;
  itemfinder: ExportedFieldItemRule;
  squirtbottle: ExportedFieldItemRule;
  coin_case: ExportedFieldItemRule;
  blue_card: ExportedFieldItemRule;
  town_map: ExportedFieldItemRule;
};

const stripComment = (line: string): string => line.replace(/;.*/, "").trim();

const parseAsmNumber = (token: string): number => {
  const trimmed = token.trim();
  if (/^\$[0-9a-f]+$/i.test(trimmed)) return Number.parseInt(trimmed.slice(1), 16);
  if (/^[0-9]+$/.test(trimmed)) return Number.parseInt(trimmed, 10);
  throw new Error(`Unsupported ASM number '${token}'.`);
};

const parseDefEquConstants = (content: string, prefix: string): Map<string, number> => {
  const constants = new Map<string, number>();
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    const match = line.match(new RegExp(`^DEF\\s+(${prefix}[A-Z0-9_]+)\\s+EQU\\s+(\\$[0-9a-fA-F]+|[0-9]+)$`));
    if (match) constants.set(match[1], parseAsmNumber(match[2]));
  }
  if (constants.size === 0) throw new Error(`No ${prefix} constants found.`);
  return constants;
};

const parseConstBlockValues = (content: string, marker: string, endPrefix: string): Map<string, number> => {
  const constants = new Map<string, number>();
  let inBlock = false;
  let value = 0;
  for (const rawLine of content.split(/\r?\n/)) {
    if (rawLine.trim() === marker) {
      inBlock = true;
      value = 0;
      continue;
    }
    if (!inBlock) continue;
    const line = stripComment(rawLine);
    const match = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (match) {
      constants.set(match[1], value);
      value += 1;
      continue;
    }
    if (line.startsWith(endPrefix)) break;
  }
  if (constants.size === 0) throw new Error(`No constants found after ${marker}.`);
  return constants;
};

const parseBadge = (badges: Map<string, number>, engineFlag: string): ExportedFieldMoveBadgeRequirement => {
  const badgeName = engineFlag.replace(/^ENGINE_/, "");
  const index = badges.get(badgeName);
  if (index === undefined) throw new Error(`No Johto badge constant for ${engineFlag}.`);
  return { region: "johto", index };
};

const tilesetName = (token: string): string => token.replace(/^TILESET_/, "").toLowerCase();

const animationVariant = (animation: number): string => {
  if (animation === 1) return "grass";
  if (animation === 0) return "tree";
  throw new Error(`Unsupported field move animation value ${animation}.`);
};

const parseReplacementTables = (
  content: string,
  rootLabel: string
): Record<string, Record<string, ExportedFieldMoveReplacement>> => {
  const lines = content.split(/\r?\n/);
  const pointers = new Map<string, string>();
  const rootIndex = lines.findIndex((line) => stripComment(line) === `${rootLabel}:`);
  if (rootIndex < 0) throw new Error(`Missing ${rootLabel}.`);
  let index = rootIndex + 1;
  for (; index < lines.length; index += 1) {
    const line = stripComment(lines[index]);
    if (line === "db -1") break;
    if (line.length === 0) continue;
    const match = line.match(/^dbw\s+(TILESET_[A-Z0-9_]+),\s+\.([a-z0-9_]+)$/);
    if (!match) throw new Error(`Unsupported ${rootLabel} pointer row '${line}'.`);
    pointers.set(match[2], tilesetName(match[1]));
  }

  const replacements: Record<string, Record<string, ExportedFieldMoveReplacement>> = {};
  for (const [label, tileset] of pointers) {
    let tableIndex = lines.findIndex((line, lineIndex) => lineIndex > index && stripComment(line) === `.${label}:`);
    if (tableIndex < 0) throw new Error(`Missing ${rootLabel} table .${label}.`);
    for (tableIndex += 1; tableIndex < lines.length; tableIndex += 1) {
      const line = stripComment(lines[tableIndex]);
      if (line === "db -1") break;
      if (line.length === 0) continue;
      const match = line.match(/^db\s+([^,]+),\s+([^,]+),\s+([^,]+)$/);
      if (!match) throw new Error(`Unsupported ${rootLabel} row '${line}'.`);
      const animation = parseAsmNumber(match[3]);
      const blockId = parseAsmNumber(match[1]);
      replacements[tileset] ??= {};
      replacements[tileset][String(blockId)] = {
        replacement_block_id: parseAsmNumber(match[2]),
        variant: rootLabel === "WhirlpoolBlockPointers" ? "whirlpool" : animationVariant(animation),
      };
    }
  }
  return replacements;
};

const requireCollision = (constants: Map<string, number>, name: string): number => {
  const value = constants.get(name);
  if (value === undefined) throw new Error(`Missing collision constant ${name}.`);
  return value;
};

const requireMoveConstant = (content: string, name: string): string => {
  const pattern = new RegExp(`^\\s*const\\s+${name}\\b`, "m");
  if (!pattern.test(content)) throw new Error(`Missing move constant ${name}.`);
  return name;
};

const labelBlock = (content: string, label: string): string => {
  const lines = content.split(/\r?\n/);
  const start = lines.findIndex((line) => stripComment(line) === `${label}:`);
  if (start < 0) throw new Error(`Missing ${label}.`);
  const block: string[] = [];
  for (let index = start + 1; index < lines.length; index += 1) {
    const line = lines[index];
    if (/^[A-Za-z0-9_.]+:\s*$/.test(stripComment(line))) break;
    block.push(line);
  }
  return block.join("\n");
};

const requireEscapeRopeRule = (itemEffectsContent: string, overworldContent: string): ExportedFieldEscapeItemRule => {
  if (!/^\s*dw\s+EscapeRopeEffect\s*;\s*ESCAPE_ROPE\b/m.test(itemEffectsContent)) {
    throw new Error("missing authored Escape Rope item effect owner");
  }
  const effectBlock = labelBlock(itemEffectsContent, "EscapeRopeEffect");
  if (!/^\s*farcall\s+EscapeRopeFunction/m.test(effectBlock)) {
    throw new Error("missing authored Escape Rope farcall");
  }
  const functionBlock = labelBlock(overworldContent, "EscapeRopeFunction");
  if (!/^\s*ld\s+a,\s*\$1\s*\n\s*jr\s+EscapeRopeOrDig/m.test(functionBlock)) {
    throw new Error("missing authored Escape Rope warp type");
  }
  const sharedBlock = labelBlock(overworldContent, "EscapeRopeOrDig");
  if (!/^\s*ld\s+\[wEscapeRopeOrDigType\],\s*a/m.test(sharedBlock)) {
    throw new Error("missing authored Escape Rope/Dig type storage");
  }
  const warpScriptBlock = labelBlock(overworldContent, ".UsedDigOrEscapeRopeScript");
  if (!/^\s*special\s+WarpToSpawnPoint/m.test(warpScriptBlock)) {
    throw new Error("missing authored Escape Rope/Dig spawn warp");
  }
  return { item_id: "ESCAPE_ROPE", escape_rope_mode: "DIG_WARP" };
};

const requireRepelRule = (itemEffectsContent: string): ExportedFieldRepelItemRule => {
  const effects = [
    ["RepelEffect", "REPEL"],
    ["SuperRepelEffect", "SUPER_REPEL"],
    ["MaxRepelEffect", "MAX_REPEL"],
  ] as const;
  for (const [label, itemEffect] of effects) {
    if (!new RegExp(`^\\s*dw\\s+${label}\\s*;\\s*${itemEffect}\\b`, "m").test(itemEffectsContent)) {
      throw new Error(`missing authored ${itemEffect} item effect owner`);
    }
    const block = labelBlock(itemEffectsContent, label);
    if (!/^\s*ld\s+b,\s*([^\s;]+)/m.test(block)) {
      throw new Error(`missing authored repel step count in ${label}`);
    }
  }
  return {};
};

const requireBicycleRule = (itemEffectsContent: string): ExportedFieldItemRule => {
  requireItemEffectOwner(itemEffectsContent, "BicycleEffect", "BICYCLE");
  const block = labelBlock(itemEffectsContent, "BicycleEffect");
  if (!/^\s*farcall\s+BikeFunction/m.test(block)) {
    throw new Error("missing authored Bicycle farcall");
  }
  return { item_id: "BICYCLE" };
};

const requireItemEffectOwner = (itemEffectsContent: string, label: string, effect: string): void => {
  if (!new RegExp(`^\\s*dw\\s+${label}\\s*;\\s*${effect}\\b`, "m").test(itemEffectsContent)) {
    throw new Error(`missing authored ${effect} item effect owner`);
  }
};

const requireFarcallItemEffectRule = (
  itemEffectsContent: string,
  label: string,
  itemId: string,
  farcall: string
): ExportedFieldItemRule => {
  requireItemEffectOwner(itemEffectsContent, label, itemId);
  const block = labelBlock(itemEffectsContent, label);
  if (!new RegExp(`^\\s*farcall\\s+${farcall}\\b`, "m").test(block)) {
    throw new Error(`missing authored ${itemId} farcall`);
  }
  return { item_id: itemId };
};

const requireTextboxItemEffectRule = (
  itemEffectsContent: string,
  label: string,
  itemId: string,
  textLabel: string
): ExportedFieldItemRule => {
  requireItemEffectOwner(itemEffectsContent, label, itemId);
  const block = labelBlock(itemEffectsContent, label);
  if (
    !new RegExp(`^\\s*ld\\s+hl,\\s*\\.${textLabel}\\b`, "m").test(block) ||
    !/^\s*jp\s+MenuTextboxWaitButton\b/m.test(block)
  ) {
    throw new Error(`missing authored ${itemId} text menu effect`);
  }
  return { item_id: itemId };
};

export function exportFieldMoves(): ExportedFieldMoveCatalog {
  const root = getDisassemblyRoot();
  const ramConstants = fs.readFileSync(path.join(root, "constants", "ram_constants.asm"), "utf8");
  const moveConstants = fs.readFileSync(path.join(root, "constants", "move_constants.asm"), "utf8");
  const itemEffects = fs.readFileSync(path.join(root, "engine", "items", "item_effects.asm"), "utf8");
  const overworld = fs.readFileSync(path.join(root, "engine", "events", "overworld.asm"), "utf8");
  const collisionConstants = parseDefEquConstants(
    fs.readFileSync(path.join(root, "constants", "collision_constants.asm"), "utf8"),
    "COLL_"
  );
  const badges = parseConstBlockValues(ramConstants, "; wJohtoBadges::", "DEF NUM_JOHTO_BADGES");
  const fieldMoveBlocks = fs.readFileSync(path.join(root, "data", "collision", "field_move_blocks.asm"), "utf8");

  const catalog: ExportedFieldMoveCatalog = {
    cut: {
      move_id: "CUT",
      badge: parseBadge(badges, "ENGINE_HIVEBADGE"),
      target_collisions: [
        "COLL_CUT_TREE",
        "COLL_CUT_TREE_1A",
        "COLL_TALL_GRASS",
        "COLL_LONG_GRASS",
        "COLL_LONG_GRASS_1C",
      ].map((name) => requireCollision(collisionConstants, name)),
      replacements: parseReplacementTables(fieldMoveBlocks, "CutTreeBlockPointers"),
    },
    whirlpool: {
      move_id: "WHIRLPOOL",
      badge: parseBadge(badges, "ENGINE_GLACIERBADGE"),
      target_collisions: ["COLL_WHIRLPOOL", "COLL_WHIRLPOOL_2C"].map((name) =>
        requireCollision(collisionConstants, name)
      ),
      replacements: parseReplacementTables(fieldMoveBlocks, "WhirlpoolBlockPointers"),
    },
    strength: {
      move_id: "STRENGTH",
      badge: parseBadge(badges, "ENGINE_PLAINBADGE"),
      engine_flag: "ENGINE_STRENGTH_ACTIVE",
    },
    flash: {
      move_id: "FLASH",
      badge: parseBadge(badges, "ENGINE_ZEPHYRBADGE"),
      engine_flag: "STATUSFLAGS_FLASH",
    },
    surf: {
      move_id: "SURF",
      badge: parseBadge(badges, "ENGINE_FOGBADGE"),
      blocked_collisions: [
        "COLL_WHIRLPOOL",
        "COLL_WHIRLPOOL_2C",
        "COLL_WATERFALL",
        "COLL_WATERFALL_RIGHT",
        "COLL_WATERFALL_LEFT",
        "COLL_WATERFALL_UP",
      ].map((name) => requireCollision(collisionConstants, name)),
      target_collisions: [],
    },
    waterfall: {
      move_id: "WATERFALL",
      badge: parseBadge(badges, "ENGINE_RISINGBADGE"),
      blocked_collisions: [],
      target_collisions: [
        "COLL_WATERFALL",
        "COLL_WATERFALL_RIGHT",
        "COLL_WATERFALL_LEFT",
        "COLL_WATERFALL_UP",
        "COLL_CURRENT_DOWN",
      ].map((name) => requireCollision(collisionConstants, name)),
    },
    fly: {
      move_id: requireMoveConstant(moveConstants, "FLY"),
      badge: parseBadge(badges, "ENGINE_STORMBADGE"),
    },
    dig: {
      move_id: requireMoveConstant(moveConstants, "DIG"),
    },
    teleport: {
      move_id: requireMoveConstant(moveConstants, "TELEPORT"),
    },
    escape_rope: requireEscapeRopeRule(itemEffects, overworld),
    repel: requireRepelRule(itemEffects),
    bicycle: requireBicycleRule(itemEffects),
    itemfinder: requireFarcallItemEffectRule(itemEffects, "ItemfinderEffect", "ITEMFINDER", "ItemFinder"),
    squirtbottle: requireFarcallItemEffectRule(itemEffects, "SquirtbottleEffect", "SQUIRTBOTTLE", "_Squirtbottle"),
    coin_case: requireTextboxItemEffectRule(itemEffects, "CoinCaseEffect", "COIN_CASE", "CoinCaseCountText"),
    blue_card: requireTextboxItemEffectRule(itemEffects, "BlueCardEffect", "BLUE_CARD", "BlueCardBalanceText"),
    town_map: requireFarcallItemEffectRule(itemEffects, "TownMapEffect", "TOWN_MAP", "PokegearMap"),
  };

  writeJsonToTargets("field_moves.json", catalog, { indent: 2 });
  return catalog;
}
