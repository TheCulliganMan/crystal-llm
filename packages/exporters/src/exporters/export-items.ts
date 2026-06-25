import fs from "fs";
import path from "path";
import { itemEffectsByAsmName } from "@pokecrystal/assets/content/items";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { parseAsmNumber, writeJsonToTargets } from "./asm-utils";

export type ExportedItem = {
  name: string;
  script_name: string;
  effect: string;
  price: number;
  held_effect: string;
  parameter: number;
  property: string;
  pocket: string;
  field_menu: string;
  field_usable: boolean;
  battle_menu: string;
  battle_usable: boolean;
  description: string;
  consumable: boolean;
  status_heals: string[];
  revive_hp_percent: number | null;
  party_revive_hp_percent: number | null;
  pp_restore_scope: string | null;
  pp_restore_points: number | null;
  pp_up_stages: number | null;
  vitamin_stat: string | null;
  vitamin_stat_exp: number | null;
  vitamin_max_stat_exp: number | null;
  rare_candy_level_gain: number | null;
  battle_stat_boost_stat: string | null;
  battle_stat_boost_stages: number | null;
  battle_escape_mode: string | null;
  battle_focus_energy: boolean | null;
  battle_stat_drop_guard: boolean | null;
  battle_stat_drop_guard_turns: number | null;
  confusion_heal: boolean | null;
  repel_steps: number | null;
  escape_rope_mode: string | null;
  tmhm_index: number | null;
  tmhm_move: string | null;
};

const SPECIAL_ITEM_OVERRIDES: Record<number, ExportedItem> = {
  0xfa: {
    name: "POKEGEAR",
    script_name: "POKEGEAR",
    effect: "NONE",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    field_usable: true,
    battle_menu: "ITEMMENU_NOUSE",
    battle_usable: false,
    description: "A versatile gadget that combines map and phone features.",
    consumable: false,
    status_heals: [],
    revive_hp_percent: null,
    party_revive_hp_percent: null,
    pp_restore_scope: null,
    pp_restore_points: null,
    pp_up_stages: null,
    vitamin_stat: null,
    vitamin_stat_exp: null,
    vitamin_max_stat_exp: null,
    rare_candy_level_gain: null,
    battle_stat_boost_stat: null,
    battle_stat_boost_stages: null,
    battle_escape_mode: null,
    battle_focus_energy: null,
    battle_stat_drop_guard: null,
    battle_stat_drop_guard_turns: null,
    confusion_heal: null,
    repel_steps: null,
    escape_rope_mode: null,
    tmhm_index: null,
    tmhm_move: null,
  },
  0xfb: {
    name: "MAP CARD",
    script_name: "MAP_CARD",
    effect: "NONE",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    field_usable: true,
    battle_menu: "ITEMMENU_NOUSE",
    battle_usable: false,
    description: "A card that adds a region map to the Pokégear.",
    consumable: false,
    status_heals: [],
    revive_hp_percent: null,
    party_revive_hp_percent: null,
    pp_restore_scope: null,
    pp_restore_points: null,
    pp_up_stages: null,
    vitamin_stat: null,
    vitamin_stat_exp: null,
    vitamin_max_stat_exp: null,
    rare_candy_level_gain: null,
    battle_stat_boost_stat: null,
    battle_stat_boost_stages: null,
    battle_escape_mode: null,
    battle_focus_energy: null,
    battle_stat_drop_guard: null,
    battle_stat_drop_guard_turns: null,
    confusion_heal: null,
    repel_steps: null,
    escape_rope_mode: null,
    tmhm_index: null,
    tmhm_move: null,
  },
  0xfc: {
    name: "PHONE CARD",
    script_name: "PHONE_CARD",
    effect: "NONE",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    field_usable: true,
    battle_menu: "ITEMMENU_NOUSE",
    battle_usable: false,
    description: "A card that enables Pokégear phone calls.",
    consumable: false,
    status_heals: [],
    revive_hp_percent: null,
    party_revive_hp_percent: null,
    pp_restore_scope: null,
    pp_restore_points: null,
    pp_up_stages: null,
    vitamin_stat: null,
    vitamin_stat_exp: null,
    vitamin_max_stat_exp: null,
    rare_candy_level_gain: null,
    battle_stat_boost_stat: null,
    battle_stat_boost_stages: null,
    battle_escape_mode: null,
    battle_focus_energy: null,
    battle_stat_drop_guard: null,
    battle_stat_drop_guard_turns: null,
    confusion_heal: null,
    repel_steps: null,
    escape_rope_mode: null,
    tmhm_index: null,
    tmhm_move: null,
  },
  0xfd: {
    name: "RADIO CARD",
    script_name: "RADIO_CARD",
    effect: "NONE",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    field_usable: true,
    battle_menu: "ITEMMENU_NOUSE",
    battle_usable: false,
    description: "A card that lets the Pokégear tune into radio stations.",
    consumable: false,
    status_heals: [],
    revive_hp_percent: null,
    party_revive_hp_percent: null,
    pp_restore_scope: null,
    pp_restore_points: null,
    pp_up_stages: null,
    vitamin_stat: null,
    vitamin_stat_exp: null,
    vitamin_max_stat_exp: null,
    rare_candy_level_gain: null,
    battle_stat_boost_stat: null,
    battle_stat_boost_stages: null,
    battle_escape_mode: null,
    battle_focus_energy: null,
    battle_stat_drop_guard: null,
    battle_stat_drop_guard_turns: null,
    confusion_heal: null,
    repel_steps: null,
    escape_rope_mode: null,
    tmhm_index: null,
    tmhm_move: null,
  },
  0xfe: {
    name: "EXPN CARD",
    script_name: "EXPN_CARD",
    effect: "NONE",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    field_usable: true,
    battle_menu: "ITEMMENU_NOUSE",
    battle_usable: false,
    description: "A card expanding the Pokégear radio with special programs.",
    consumable: false,
    status_heals: [],
    revive_hp_percent: null,
    party_revive_hp_percent: null,
    pp_restore_scope: null,
    pp_restore_points: null,
    pp_up_stages: null,
    vitamin_stat: null,
    vitamin_stat_exp: null,
    vitamin_max_stat_exp: null,
    rare_candy_level_gain: null,
    battle_stat_boost_stat: null,
    battle_stat_boost_stages: null,
    battle_escape_mode: null,
    battle_focus_energy: null,
    battle_stat_drop_guard: null,
    battle_stat_drop_guard_turns: null,
    confusion_heal: null,
    repel_steps: null,
    escape_rope_mode: null,
    tmhm_index: null,
    tmhm_move: null,
  },
};

const ITEM_SLOT_COUNT = 0x100;

function parsePrice(value: string): number {
  const trimmed = value.trim();
  if (!trimmed) return 0;
  if (trimmed.startsWith("$")) {
    return Number.parseInt(trimmed.slice(1), 10);
  }
  return parseAsmNumber(trimmed);
}

function parseAttributes(content: string): Array<Record<string, string | number>> {
  const items: Array<Record<string, string | number>> = [];
  const pattern =
    /; (.*?)\n\s*item_attribute\s*(.*?),\s*(.*?),\s*(.*?),\s*(.*?),\s*(.*?),\s*(.*?),\s*(.*)/g;
  for (const match of content.matchAll(pattern)) {
    items.push({
      name: match[1].trim().replace(/_/g, " "),
      source_name: match[1].trim(),
      price: parsePrice(match[2]),
      held_effect: match[3],
      parameter: parseAsmNumber(match[4]),
      property: match[5],
      pocket: match[6],
      field_menu: match[7],
      battle_menu: match[8],
    });
  }
  return items;
}

function parseDescriptions(content: string): Record<string, string> {
  const descriptions: Record<string, string> = {};
  const blocks = content.split(/(\w+Desc:)/g);
  for (let index = 1; index < blocks.length; index += 2) {
    const label = blocks[index].replace(":", "");
    const blockContent = blocks[index + 1] ?? "";
    const textParts = [...blockContent.matchAll(/"(.*?)"/g)].map((match) => match[1]);
    descriptions[label] = textParts.join(" ").replace(/@/g, "").trim();
  }
  return descriptions;
}

function parseDescriptionPointers(content: string): string[] {
  const labels: string[] = [];
  let inTable = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.replace(/;.*/, "").trim();
    if (line === "ItemDescriptions:") {
      inTable = true;
      continue;
    }
    if (!inTable) continue;
    if (/^\w+Desc:$/.test(line)) break;
    const pointer = line.match(/^dw\s+(\w+Desc)$/);
    if (pointer) labels.push(pointer[1]);
  }
  return labels;
}

function parseTmHmSymbols(content: string): Record<string, { script_name: string; tmhm_index: number; tmhm_move: string }> {
  const symbols: Record<string, { script_name: string; tmhm_index: number; tmhm_move: string }> = {};
  let tmNumber = 1;
  let hmNumber = 1;
  let tmhmIndex = 0;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.replace(/;.*/, "").trim();
    const tm = line.match(/^add_tm\s+([A-Z0-9_]+)$/);
    if (tm) {
      symbols[`TM${String(tmNumber).padStart(2, "0")}`] = {
        script_name: `TM_${tm[1]}`,
        tmhm_index: tmhmIndex,
        tmhm_move: tm[1],
      };
      tmNumber += 1;
      tmhmIndex += 1;
      continue;
    }
    const hm = line.match(/^add_hm\s+([A-Z0-9_]+)$/);
    if (hm) {
      symbols[`HM${String(hmNumber).padStart(2, "0")}`] = {
        script_name: `HM_${hm[1]}`,
        tmhm_index: tmhmIndex,
        tmhm_move: hm[1],
      };
      hmNumber += 1;
      tmhmIndex += 1;
    }
  }
  return symbols;
}

function isUnusedAsmItemSlot(attributes: Record<string, string | number>): boolean {
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  return (
    (/^ITEM_[0-9A-Z_]+$/.test(sourceName) || /^\$[0-9a-f]+$/i.test(sourceName)) &&
    attributes.held_effect === "HELD_NONE" &&
    attributes.parameter === 0 &&
    attributes.property === "NO_LIMITS" &&
    attributes.pocket === "ITEM" &&
    attributes.field_menu === "ITEMMENU_NOUSE" &&
    attributes.battle_menu === "ITEMMENU_NOUSE"
  );
}

function exactAuthoredItemEffect(attributes: Record<string, string | number>): string | undefined {
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  if (!sourceName) return undefined;
  return itemEffectsByAsmName.get(sourceName);
}

function isConsumableItem(attributes: Record<string, string | number>, tmhmIndex: number | null): boolean {
  const pocket = String(attributes.pocket);
  if (pocket === "BALL") return true;
  if (pocket === "KEY_ITEM") return false;
  if (pocket === "TM_HM") return tmhmIndex !== null && tmhmIndex < 50;
  if (pocket !== "ITEM") return false;
  const fieldMenu = String(attributes.field_menu);
  const battleMenu = String(attributes.battle_menu);
  return fieldMenu !== "ITEMMENU_NOUSE" || battleMenu !== "ITEMMENU_NOUSE";
}

function parseAsmConstants(content: string): Map<string, number> {
  const constants = new Map<string, number>();
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.replace(/;.*/, "").trim();
    const match = line.match(/^DEF\s+([A-Z0-9_]+)\s+EQU\s+(.+)$/);
    if (!match) continue;
    constants.set(match[1], parseAsmNumber(match[2]));
  }
  return constants;
}

function parseAsmValue(value: string, constants: Map<string, number>): number {
  const trimmed = value.trim();
  const constant = constants.get(trimmed);
  if (constant !== undefined) return constant;
  return parseAsmNumber(trimmed);
}

function parseHealingHpAmounts(content: string, constants: Map<string, number>): Map<string, number> {
  const maxStatValue = constants.get("MAX_STAT_VALUE");
  if (maxStatValue === undefined) {
    throw new Error("missing ASM constant MAX_STAT_VALUE for healing HP amounts");
  }
  const table = new Map<string, number>();
  const block = labelBlock(content, "HealingHPAmounts");
  for (const rawLine of block.split(/\r?\n/)) {
    const line = rawLine.replace(/;.*/, "").trim();
    if (!line) continue;
    const match = line.match(/^dbw\s+([^,\s]+)\s*,\s*(.+)$/);
    if (!match) continue;
    if (match[1] === "-1") break;
    const amount = parseAsmValue(match[2], constants);
    table.set(match[1], amount === maxStatValue ? -1 : amount);
  }
  return table;
}

function exactItemParameter(
  attributes: Record<string, string | number>,
  effect: string,
  healingHpAmounts: Map<string, number>
): number {
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  if (effect !== "RESTORE_HP" && effect !== "FULL_RESTORE") return Number(attributes.parameter);
  const parameter = healingHpAmounts.get(sourceName);
  if (parameter === undefined) {
    throw new Error(`missing authored HP restore parameter for item ${sourceName}`);
  }
  return parameter;
}

function parseStatusConstants(content: string, constants: Map<string, number>): Map<string, number> {
  const statusConstants = new Map(constants);
  let constValue = 0;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.replace(/;.*/, "").trim();
    if (!line) continue;
    const constDefMatch = line.match(/^const_def(?:\s+(.+))?$/);
    if (constDefMatch) {
      constValue = constDefMatch[1] ? parseAsmValue(constDefMatch[1], statusConstants) : 0;
      continue;
    }
    const constMatch = line.match(/^const\s+([A-Z0-9_]+)$/);
    if (!constMatch) continue;
    statusConstants.set(constMatch[1], constValue);
    constValue += 1;
  }
  return statusConstants;
}

function parseStatusMaskExpression(expression: string, constants: Map<string, number>): number {
  const trimmed = expression.trim();
  const shiftMatch = trimmed.match(/^1\s*<<\s*([A-Z0-9_]+)$/);
  if (shiftMatch) {
    const bit = constants.get(shiftMatch[1]);
    if (bit === undefined) {
      throw new Error(`missing ASM status constant ${shiftMatch[1]}`);
    }
    return 1 << bit;
  }
  return parseAsmValue(trimmed, constants);
}

function statusHealsFromMask(mask: number, constants: Map<string, number>): string[] {
  const slpMask = constants.get("SLP_MASK");
  const psn = constants.get("PSN");
  const brn = constants.get("BRN");
  const frz = constants.get("FRZ");
  const par = constants.get("PAR");
  if (slpMask === undefined || psn === undefined || brn === undefined || frz === undefined || par === undefined) {
    throw new Error("missing ASM status constants for status healing actions");
  }
  const statuses: string[] = [];
  if ((mask & (1 << psn)) !== 0) statuses.push("POISON");
  if ((mask & (1 << brn)) !== 0) statuses.push("BURN");
  if ((mask & (1 << frz)) !== 0) statuses.push("FREEZE");
  if ((mask & slpMask) !== 0) statuses.push("SLEEP");
  if ((mask & (1 << par)) !== 0) statuses.push("PARALYSIS");
  return statuses;
}

function parseStatusHealRules(statusHealContent: string, constants: Map<string, number>): Map<string, string[]> {
  const rules = new Map<string, string[]>();
  const table = labelBlock(statusHealContent, "StatusHealingActions");
  for (const rawLine of table.split(/\r?\n/)) {
    const line = rawLine.replace(/;.*/, "").trim();
    if (!line) continue;
    const match = line.match(/^db\s+([^,\s]+)\s*,\s*([^,\s]+)\s*,\s*(.+)$/);
    if (!match) continue;
    if (match[1] === "-1") break;
    const mask = parseStatusMaskExpression(match[3], constants);
    rules.set(match[1], statusHealsFromMask(mask, constants));
  }
  return rules;
}

function exactStatusHeals(
  attributes: Record<string, string | number>,
  effect: string,
  statusHealRules: Map<string, string[]>
): string[] {
  if (effect === "FULL_RESTORE") return ["POISON", "BURN", "FREEZE", "SLEEP", "PARALYSIS"];
  if (effect !== "STATUS_HEAL") return [];
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const statuses = statusHealRules.get(sourceName);
  if (!statuses) {
    throw new Error(`missing authored status_heals for status item ${sourceName}`);
  }
  return [...statuses];
}

function parseItemEffectLabels(itemEffectsContent: string): Map<string, string> {
  const labels = new Map<string, string>();
  const table = labelBlock(itemEffectsContent, "ItemEffects");
  for (const match of table.matchAll(/^\s*dw\s+([A-Za-z0-9_]+)\s*;\s*([A-Z0-9_]+)/gm)) {
    labels.set(match[2], match[1]);
  }
  return labels;
}

function itemNameForEffectLabel(labels: Map<string, string>, effectLabel: string): string {
  const itemNames = [...labels.entries()]
    .filter(([, label]) => label === effectLabel)
    .map(([itemName]) => itemName);
  if (itemNames.length !== 1) {
    throw new Error(`missing unique authored item effect owner for ${effectLabel}`);
  }
  return itemNames[0];
}

function parseReviveHpPercentRules(itemEffectsContent: string): Map<string, number> {
  const labels = parseItemEffectLabels(itemEffectsContent);
  const reviveBlock = labelBlock(itemEffectsContent, "RevivePokemon");
  const halfItemMatch = reviveBlock.match(/^\s*cp\s+([A-Z0-9_]+)\s*\n\s*jr\s+z,\s*\.revive_half_hp/m);
  if (!halfItemMatch) {
    throw new Error("missing authored half-revive item branch in RevivePokemon");
  }
  const halfBlock = labelBlock(itemEffectsContent, "ReviveHalfHP");
  if (!/^\s*srl\s+d\s*\n\s*rr\s+e/m.test(halfBlock)) {
    throw new Error("missing authored half HP revive calculation in ReviveHalfHP");
  }
  const fullBlock = labelBlock(itemEffectsContent, "ReviveFullHP");
  if (!/^\s*call\s+LoadHPFromBuffer1/m.test(fullBlock)) {
    throw new Error("missing authored full HP revive calculation in ReviveFullHP");
  }
  const rules = new Map<string, number>();
  for (const [itemName, label] of labels) {
    if (label !== "ReviveEffect" && label !== "RevivalHerbEffect") continue;
    const block = labelBlock(itemEffectsContent, label);
    if (!/^\s*call\s+RevivePokemon/m.test(block)) continue;
    rules.set(itemName, itemName === halfItemMatch[1] ? 50 : 100);
  }
  return rules;
}

function exactReviveHpPercent(
  attributes: Record<string, string | number>,
  effect: string,
  reviveHpPercentRules: Map<string, number>
): number | null {
  if (effect !== "REVIVE") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const percent = reviveHpPercentRules.get(sourceName);
  if (percent === undefined) {
    throw new Error(`missing authored revive_hp_percent for revive item ${sourceName}`);
  }
  return percent;
}

function parsePartyReviveHpPercentRules(
  itemEffectsContent: string,
  sacredAshContent: string
): Map<string, number> {
  const labels = parseItemEffectLabels(itemEffectsContent);
  if (labels.get("SACRED_ASH") !== "SacredAshEffect") {
    throw new Error("missing authored Sacred Ash item effect label");
  }
  const effectBlock = labelBlock(itemEffectsContent, "SacredAshEffect");
  if (!/^\s*farcall\s+_SacredAsh/m.test(effectBlock)) {
    throw new Error("missing authored Sacred Ash farcall");
  }
  const scriptBlock = labelBlock(sacredAshContent, "SacredAshScript");
  if (!/^\s*special\s+HealParty/m.test(scriptBlock)) {
    throw new Error("missing authored Sacred Ash HealParty script");
  }
  const rules = new Map<string, number>();
  rules.set("SACRED_ASH", 100);
  return rules;
}

function exactPartyReviveHpPercent(
  attributes: Record<string, string | number>,
  effect: string,
  partyReviveHpPercentRules: Map<string, number>
): number | null {
  if (effect !== "SACRED_ASH") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const percent = partyReviveHpPercentRules.get(sourceName);
  if (percent === undefined) {
    throw new Error(`missing authored party_revive_hp_percent for party revive item ${sourceName}`);
  }
  return percent;
}

function parsePpRestoreScopeRules(itemEffectsContent: string): Map<string, string> {
  const labels = parseItemEffectLabels(itemEffectsContent);
  const effectBlock = labelBlock(itemEffectsContent, "RestorePPEffect");
  const allMoveItems = new Set<string>();
  for (const match of effectBlock.matchAll(/^\s*cp\s+([A-Z0-9_]+)\s*\n\s*jp\s+z,\s*Elixer_RestorePPofAllMoves/gm)) {
    allMoveItems.add(match[1]);
  }
  const rules = new Map<string, string>();
  for (const [itemName, label] of labels) {
    if (label !== "RestorePPEffect") continue;
    rules.set(itemName, allMoveItems.has(itemName) ? "POKEMON" : "MOVE");
  }
  return rules;
}

function exactPpRestoreScope(
  attributes: Record<string, string | number>,
  effect: string,
  ppRestoreScopeRules: Map<string, string>
): string | null {
  if (effect !== "RESTORE_PP") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const scope = ppRestoreScopeRules.get(sourceName);
  if (!scope) {
    throw new Error(`missing authored pp_restore_scope for PP item ${sourceName}`);
  }
  return scope;
}

function exactPpRestorePoints(attributes: Record<string, string | number>, effect: string): number | null {
  if (effect !== "RESTORE_PP") return null;
  const amount = Number(attributes.parameter);
  if (amount === -1) return null;
  if (amount <= 0) {
    const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
    throw new Error(`invalid authored pp_restore_points for PP item ${sourceName}`);
  }
  return amount;
}

function parsePpUpStages(constants: Map<string, number>): number {
  const ppUpOne = constants.get("PP_UP_ONE");
  if (ppUpOne === undefined) {
    throw new Error("missing ASM constant PP_UP_ONE for PP Up stages");
  }
  if (ppUpOne === 0 || (ppUpOne & (ppUpOne - 1)) !== 0) {
    throw new Error(`invalid ASM constant PP_UP_ONE ${ppUpOne}`);
  }
  return Math.log2(ppUpOne) - 5;
}

function exactPpUpStages(
  attributes: Record<string, string | number>,
  effect: string,
  ppUpStages: number
): number | null {
  if (effect !== "PP_UP") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  if (sourceName !== "PP_UP") {
    throw new Error(`missing authored pp_up_stages for PP Up item ${sourceName}`);
  }
  return ppUpStages;
}

type VitaminRule = {
  stat: string;
  statExp: number;
  maxStatExp: number;
};

function vitaminStatFromAsmOffset(offsetExpression: string): string {
  const match = offsetExpression.match(/^(MON_[A-Z]+_EXP)\s*-\s*MON_STAT_EXP$/);
  if (!match) {
    throw new Error(`unsupported vitamin stat exp offset '${offsetExpression}'`);
  }
  switch (match[1]) {
    case "MON_HP_EXP":
      return "HP";
    case "MON_ATK_EXP":
      return "ATTACK";
    case "MON_DEF_EXP":
      return "DEFENSE";
    case "MON_SPD_EXP":
      return "SPEED";
    case "MON_SPC_EXP":
      return "SPECIAL";
    default:
      throw new Error(`unsupported vitamin stat exp field '${match[1]}'`);
  }
}

function parseVitaminRules(itemEffectsContent: string): Map<string, VitaminRule> {
  const effectBlock = labelBlock(itemEffectsContent, "VitaminEffect");
  const maxHighByteMatch = effectBlock.match(/^\s*cp\s+([-+$%0-9][^\s;]*)/m);
  const gainHighByteMatch = effectBlock.match(/^\s*add\s+([-+$%0-9][^\s;]*)/m);
  if (!maxHighByteMatch) {
    throw new Error("missing authored vitamin stat exp cap in VitaminEffect");
  }
  if (!gainHighByteMatch) {
    throw new Error("missing authored vitamin stat exp gain in VitaminEffect");
  }
  const maxStatExp = parseAsmNumber(maxHighByteMatch[1]) * 256;
  const statExp = parseAsmNumber(gainHighByteMatch[1]) * 256;
  const rules = new Map<string, VitaminRule>();
  const table = labelBlock(itemEffectsContent, "StatExpItemPointerOffsets");
  for (const rawLine of table.split(/\r?\n/)) {
    const line = rawLine.replace(/;.*/, "").trim();
    if (!line) continue;
    const match = line.match(/^db\s+([^,\s]+)\s*,\s*(.+)$/);
    if (!match) continue;
    rules.set(match[1], {
      stat: vitaminStatFromAsmOffset(match[2].trim()),
      statExp,
      maxStatExp,
    });
  }
  return rules;
}

function exactVitaminRule(
  attributes: Record<string, string | number>,
  effect: string,
  vitaminRules: Map<string, VitaminRule>
): VitaminRule | null {
  if (effect !== "VITAMIN") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const rule = vitaminRules.get(sourceName);
  if (!rule) {
    throw new Error(`missing authored vitamin_stat for vitamin item ${sourceName}`);
  }
  return rule;
}

function exactVitaminStat(
  attributes: Record<string, string | number>,
  effect: string,
  vitaminRules: Map<string, VitaminRule>
): string | null {
  return exactVitaminRule(attributes, effect, vitaminRules)?.stat ?? null;
}

function exactVitaminStatExp(
  attributes: Record<string, string | number>,
  effect: string,
  vitaminRules: Map<string, VitaminRule>
): number | null {
  return exactVitaminRule(attributes, effect, vitaminRules)?.statExp ?? null;
}

function exactVitaminMaxStatExp(
  attributes: Record<string, string | number>,
  effect: string,
  vitaminRules: Map<string, VitaminRule>
): number | null {
  return exactVitaminRule(attributes, effect, vitaminRules)?.maxStatExp ?? null;
}

function parseRareCandyLevelGain(itemEffectsContent: string): number {
  const block = labelBlock(itemEffectsContent, "RareCandyEffect");
  if (!/^\s*inc\s+a\s*\n\s*ld\s+\[hl\],\s*a/m.test(block)) {
    throw new Error("missing authored Rare Candy level increment in RareCandyEffect");
  }
  return 1;
}

function exactRareCandyLevelGain(
  attributes: Record<string, string | number>,
  effect: string,
  rareCandyLevelGain: number
): number | null {
  if (effect !== "RARE_CANDY") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  if (sourceName !== "RARE_CANDY") {
    throw new Error(`missing authored rare_candy_level_gain for rare candy item ${sourceName}`);
  }
  return rareCandyLevelGain;
}

const BATTLE_STAT_BOOST_STAGES = 1;

function battleStatFromAsmStat(stat: string): string {
  switch (stat) {
    case "ATTACK":
      return "ATTACK";
    case "DEFENSE":
      return "DEFENSE";
    case "SPEED":
      return "SPEED";
    case "SP_ATTACK":
      return "SPECIAL_ATTACK";
    case "ACCURACY":
      return "ACCURACY";
    default:
      throw new Error(`unsupported battle stat boost stat '${stat}'`);
  }
}

function parseXItemStatRules(content: string): Map<string, string> {
  const rules = new Map<string, string>();
  const block = labelBlock(content, "XItemStats");
  for (const rawLine of block.split(/\r?\n/)) {
    const line = rawLine.replace(/;.*/, "").trim();
    if (!line) continue;
    const match = line.match(/^db\s+([^,\s]+)\s*,\s*(.+)$/);
    if (!match) continue;
    rules.set(match[1], battleStatFromAsmStat(match[2].trim()));
  }
  return rules;
}

function exactBattleStatBoostStat(
  attributes: Record<string, string | number>,
  effect: string,
  xItemStatRules: Map<string, string>
): string | null {
  if (effect !== "X_ITEM" && effect !== "X_ACCURACY") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const stat = effect === "X_ACCURACY" ? "ACCURACY" : xItemStatRules.get(sourceName);
  if (!stat) {
    throw new Error(`missing authored battle_stat_boost_stat for battle boost item ${sourceName}`);
  }
  return stat;
}

function exactBattleStatBoostStages(
  attributes: Record<string, string | number>,
  effect: string,
  xItemStatRules: Map<string, string>
): number | null {
  if (effect !== "X_ITEM" && effect !== "X_ACCURACY") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  if (effect !== "X_ACCURACY" && !xItemStatRules.has(sourceName)) {
    throw new Error(`missing authored battle_stat_boost_stages for battle boost item ${sourceName}`);
  }
  return BATTLE_STAT_BOOST_STAGES;
}

function parseBattleEscapeModeRules(itemEffectsContent: string): Map<string, string> {
  const labels = parseItemEffectLabels(itemEffectsContent);
  const itemName = itemNameForEffectLabel(labels, "PokeDollEffect");
  const block = labelBlock(itemEffectsContent, "PokeDollEffect");
  if (!/^\s*ld\s+a,\s*\[wBattleMode\]\s*\n\s*dec\s+a\s*;\s*WILD_BATTLE\?/m.test(block)) {
    throw new Error("missing authored Poke Doll wild battle check");
  }
  if (!/^\s*or\s+DRAW\s*\n\s*ld\s+\[wBattleResult\],\s*a/m.test(block)) {
    throw new Error("missing authored Poke Doll draw battle result");
  }
  return new Map([[itemName, "WILD_BATTLE"]]);
}

function exactBattleEscapeMode(
  attributes: Record<string, string | number>,
  effect: string,
  battleEscapeModeRules: Map<string, string>
): string | null {
  if (effect !== "POKE_DOLL") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const mode = battleEscapeModeRules.get(sourceName);
  if (!mode) {
    throw new Error(`missing authored battle_escape_mode for battle escape item ${sourceName}`);
  }
  return mode;
}

function parseBattleFocusEnergyRules(itemEffectsContent: string): Map<string, boolean> {
  const labels = parseItemEffectLabels(itemEffectsContent);
  const itemName = itemNameForEffectLabel(labels, "DireHitEffect");
  const block = labelBlock(itemEffectsContent, "DireHitEffect");
  if (!/^\s*set\s+SUBSTATUS_FOCUS_ENERGY,\s*\[hl\]/m.test(block)) {
    throw new Error("missing authored Dire Hit focus energy substatus");
  }
  return new Map([[itemName, true]]);
}

function exactBattleFocusEnergy(
  attributes: Record<string, string | number>,
  effect: string,
  battleFocusEnergyRules: Map<string, boolean>
): boolean | null {
  if (effect !== "DIRE_HIT") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const focusEnergy = battleFocusEnergyRules.get(sourceName);
  if (focusEnergy === undefined) {
    throw new Error(`missing authored battle_focus_energy for critical focus item ${sourceName}`);
  }
  return focusEnergy;
}

function parseBattleStatDropGuardRules(itemEffectsContent: string): Map<string, boolean> {
  const labels = parseItemEffectLabels(itemEffectsContent);
  const itemName = itemNameForEffectLabel(labels, "GuardSpecEffect");
  const block = labelBlock(itemEffectsContent, "GuardSpecEffect");
  if (!/^\s*bit\s+SUBSTATUS_MIST,\s*\[hl\]/m.test(block) || !/^\s*set\s+SUBSTATUS_MIST,\s*\[hl\]/m.test(block)) {
    throw new Error("missing authored Guard Spec mist substatus");
  }
  return new Map([[itemName, true]]);
}

function exactBattleStatDropGuard(
  attributes: Record<string, string | number>,
  effect: string,
  battleStatDropGuardRules: Map<string, boolean>
): boolean | null {
  if (effect !== "GUARD_SPEC") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const guard = battleStatDropGuardRules.get(sourceName);
  if (guard === undefined) {
    throw new Error(`missing authored battle_stat_drop_guard for stat guard item ${sourceName}`);
  }
  return guard;
}

function exactBattleStatDropGuardTurns(attributes: Record<string, string | number>, effect: string): number | null {
  if (effect !== "GUARD_SPEC") return null;
  return null;
}

function parseConfusionHealRules(itemEffectsContent: string): Map<string, boolean> {
  const labels = parseItemEffectLabels(itemEffectsContent);
  const itemName = itemNameForEffectLabel(labels, "BitterBerryEffect");
  const block = labelBlock(itemEffectsContent, "BitterBerryEffect");
  if (!/^\s*bit\s+SUBSTATUS_CONFUSED,\s*\[hl\]/m.test(block) || !/^\s*res\s+SUBSTATUS_CONFUSED,\s*\[hl\]/m.test(block)) {
    throw new Error("missing authored Bitter Berry confusion heal");
  }
  return new Map([[itemName, true]]);
}

function exactConfusionHeal(
  attributes: Record<string, string | number>,
  effect: string,
  confusionHealRules: Map<string, boolean>
): boolean | null {
  if (effect !== "BITTER_BERRY") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const heals = confusionHealRules.get(sourceName);
  if (heals === undefined) {
    throw new Error(`missing authored confusion_heal for confusion item ${sourceName}`);
  }
  return heals;
}

function labelBlock(content: string, label: string): string {
  const pattern = new RegExp(`^${label}:\\n([\\s\\S]*?)(?=^[A-Za-z_][A-Za-z0-9_]*:|$(?![\\s\\S]))`, "m");
  const match = content.match(pattern);
  if (!match) {
    throw new Error(`missing item effect label ${label}`);
  }
  return match[1];
}

function parseRepelStepRules(itemEffectsContent: string): Map<string, number> {
  const rules = new Map<string, number>();
  for (const [itemName, effectLabel] of [
    ["REPEL", "RepelEffect"],
    ["SUPER_REPEL", "SuperRepelEffect"],
    ["MAX_REPEL", "MaxRepelEffect"],
  ] as const) {
    const block = labelBlock(itemEffectsContent, effectLabel);
    const match = block.match(/^\s*ld\s+b,\s*([^\s;]+)/m);
    if (!match) {
      throw new Error(`missing authored repel step count in ${effectLabel}`);
    }
    rules.set(itemName, parseAsmNumber(match[1]));
  }
  return rules;
}

function exactRepelSteps(
  attributes: Record<string, string | number>,
  effect: string,
  repelStepRules: Map<string, number>
): number | null {
  if (effect !== "REPEL" && effect !== "SUPER_REPEL" && effect !== "MAX_REPEL") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const steps = repelStepRules.get(sourceName);
  if (steps === undefined) {
    throw new Error(`missing authored repel_steps for repel item ${sourceName}`);
  }
  return steps;
}

function parseEscapeRopeModeRules(itemEffectsContent: string, overworldContent: string): Map<string, string> {
  const labels = parseItemEffectLabels(itemEffectsContent);
  const itemName = itemNameForEffectLabel(labels, "EscapeRopeEffect");
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
  if (!/^\s*special\s+WarpToSpawnPoint/m.test(sharedBlock)) {
    throw new Error("missing authored Escape Rope/Dig spawn warp");
  }
  return new Map([[itemName, "DIG_WARP"]]);
}

function exactEscapeRopeMode(
  attributes: Record<string, string | number>,
  effect: string,
  escapeRopeModeRules: Map<string, string>
): string | null {
  if (effect !== "ESCAPE_ROPE") return null;
  const sourceName = typeof attributes.source_name === "string" ? attributes.source_name : "";
  const mode = escapeRopeModeRules.get(sourceName);
  if (!mode) {
    throw new Error(`missing authored escape_rope_mode for escape rope item ${sourceName}`);
  }
  return mode;
}

export function exportItems(): ExportedItem[] {
  const root = getDisassemblyRoot();
  const attributesPath = path.join(root, "data", "items", "attributes.asm");
  const descriptionsPath = path.join(root, "data", "items", "descriptions.asm");
  const healHpPath = path.join(root, "data", "items", "heal_hp.asm");
  const healStatusPath = path.join(root, "data", "items", "heal_status.asm");
  const xStatsPath = path.join(root, "data", "items", "x_stats.asm");
  const itemConstantsPath = path.join(root, "constants", "item_constants.asm");
  const battleConstantsPath = path.join(root, "constants", "battle_constants.asm");
  const pokemonDataConstantsPath = path.join(root, "constants", "pokemon_data_constants.asm");
  const overworldPath = path.join(root, "engine", "events", "overworld.asm");
  const sacredAshPath = path.join(root, "engine", "events", "sacred_ash.asm");
  const itemEffectsPath = path.join(root, "engine", "items", "item_effects.asm");
  const attributes = parseAttributes(fs.readFileSync(attributesPath, "utf8"));
  if (attributes.length !== ITEM_SLOT_COUNT) {
    throw new Error(`Item attribute table must contain exactly ${ITEM_SLOT_COUNT} rows, found ${attributes.length}.`);
  }
  const descriptionContent = fs.readFileSync(descriptionsPath, "utf8");
  const descriptions = parseDescriptions(descriptionContent);
  const descriptionPointers = parseDescriptionPointers(descriptionContent);
  if (descriptionPointers.length !== ITEM_SLOT_COUNT - 1) {
    throw new Error(
      `Item description pointer table must contain exactly ${ITEM_SLOT_COUNT - 1} rows, found ${descriptionPointers.length}.`
    );
  }
  const asmConstants = parseAsmConstants(fs.readFileSync(battleConstantsPath, "utf8"));
  for (const [name, value] of parseAsmConstants(fs.readFileSync(pokemonDataConstantsPath, "utf8"))) {
    asmConstants.set(name, value);
  }
  const statusConstants = parseStatusConstants(fs.readFileSync(battleConstantsPath, "utf8"), asmConstants);
  const healingHpAmounts = parseHealingHpAmounts(fs.readFileSync(healHpPath, "utf8"), asmConstants);
  const statusHealRules = parseStatusHealRules(fs.readFileSync(healStatusPath, "utf8"), statusConstants);
  const ppUpStages = parsePpUpStages(asmConstants);
  const xItemStatRules = parseXItemStatRules(fs.readFileSync(xStatsPath, "utf8"));
  const tmhmSymbols = parseTmHmSymbols(fs.readFileSync(itemConstantsPath, "utf8"));
  const itemEffectsContent = fs.readFileSync(itemEffectsPath, "utf8");
  const overworldContent = fs.readFileSync(overworldPath, "utf8");
  const sacredAshContent = fs.readFileSync(sacredAshPath, "utf8");
  const reviveHpPercentRules = parseReviveHpPercentRules(itemEffectsContent);
  const partyReviveHpPercentRules = parsePartyReviveHpPercentRules(itemEffectsContent, sacredAshContent);
  const ppRestoreScopeRules = parsePpRestoreScopeRules(itemEffectsContent);
  const battleEscapeModeRules = parseBattleEscapeModeRules(itemEffectsContent);
  const battleFocusEnergyRules = parseBattleFocusEnergyRules(itemEffectsContent);
  const battleStatDropGuardRules = parseBattleStatDropGuardRules(itemEffectsContent);
  const confusionHealRules = parseConfusionHealRules(itemEffectsContent);
  const escapeRopeModeRules = parseEscapeRopeModeRules(itemEffectsContent, overworldContent);
  const repelStepRules = parseRepelStepRules(itemEffectsContent);
  const vitaminRules = parseVitaminRules(itemEffectsContent);
  const rareCandyLevelGain = parseRareCandyLevelGain(itemEffectsContent);
  const items: ExportedItem[] = [];

  for (let index = 0; index < ITEM_SLOT_COUNT; index += 1) {
    const override = SPECIAL_ITEM_OVERRIDES[index];
    if (override) {
      items.push(override);
      continue;
    }
    const attr = attributes[index];
    const effect =
      typeof attr.pocket === "string" && attr.pocket === "TM_HM"
        ? "NONE"
        : isUnusedAsmItemSlot(attr)
          ? "NONE"
          : exactAuthoredItemEffect(attr);
    if (!effect) {
      throw new Error(`missing authored item effect for item slot ${index}`);
    }
    const descriptionLabel = descriptionPointers[index];
    const description = descriptionLabel ? descriptions[descriptionLabel] : "";
    if (index < ITEM_SLOT_COUNT - 1 && description === undefined) {
      throw new Error(`missing item description label ${descriptionLabel} for item slot ${index}`);
    }
    const tmhmSymbol =
      typeof attr.source_name === "string" ? tmhmSymbols[attr.source_name] : undefined;
    const tmhmIndex = tmhmSymbol?.tmhm_index ?? null;
    const tmhmMove = tmhmSymbol?.tmhm_move ?? null;
    const effectId = String(effect);
    items.push({
      name: String(attr.name),
      script_name: tmhmSymbol?.script_name ?? String(attr.source_name),
      effect: String(effect),
      price: Number(attr.price),
      held_effect: String(attr.held_effect),
      parameter: exactItemParameter(attr, effectId, healingHpAmounts),
      property: String(attr.property),
      pocket: String(attr.pocket),
      field_menu: String(attr.field_menu),
      field_usable: String(attr.field_menu) !== "ITEMMENU_NOUSE",
      battle_menu: String(attr.battle_menu),
      battle_usable: String(attr.battle_menu) !== "ITEMMENU_NOUSE",
      description,
      consumable: isConsumableItem(attr, tmhmIndex),
      status_heals: exactStatusHeals(attr, effectId, statusHealRules),
      revive_hp_percent: exactReviveHpPercent(attr, effectId, reviveHpPercentRules),
      party_revive_hp_percent: exactPartyReviveHpPercent(attr, effectId, partyReviveHpPercentRules),
      pp_restore_scope: exactPpRestoreScope(attr, effectId, ppRestoreScopeRules),
      pp_restore_points: exactPpRestorePoints(attr, effectId),
      pp_up_stages: exactPpUpStages(attr, effectId, ppUpStages),
      vitamin_stat: exactVitaminStat(attr, effectId, vitaminRules),
      vitamin_stat_exp: exactVitaminStatExp(attr, effectId, vitaminRules),
      vitamin_max_stat_exp: exactVitaminMaxStatExp(attr, effectId, vitaminRules),
      rare_candy_level_gain: exactRareCandyLevelGain(attr, effectId, rareCandyLevelGain),
      battle_stat_boost_stat: exactBattleStatBoostStat(attr, effectId, xItemStatRules),
      battle_stat_boost_stages: exactBattleStatBoostStages(attr, effectId, xItemStatRules),
      battle_escape_mode: exactBattleEscapeMode(attr, effectId, battleEscapeModeRules),
      battle_focus_energy: exactBattleFocusEnergy(attr, effectId, battleFocusEnergyRules),
      battle_stat_drop_guard: exactBattleStatDropGuard(attr, effectId, battleStatDropGuardRules),
      battle_stat_drop_guard_turns: exactBattleStatDropGuardTurns(attr, effectId),
      confusion_heal: exactConfusionHeal(attr, effectId, confusionHealRules),
      repel_steps: exactRepelSteps(attr, effectId, repelStepRules),
      escape_rope_mode: exactEscapeRopeMode(attr, effectId, escapeRopeModeRules),
      tmhm_index: tmhmIndex,
      tmhm_move: tmhmMove,
    });
  }

  writeJsonToTargets("items.json", items, { indent: 2 });
  return items;
}
