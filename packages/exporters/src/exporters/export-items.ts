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
  battle_menu: string;
  description: string;
  consumable: boolean;
  tmhm_index: number | null;
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
    battle_menu: "ITEMMENU_NOUSE",
    description: "A versatile gadget that combines map and phone features.",
    consumable: false,
    tmhm_index: null,
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
    battle_menu: "ITEMMENU_NOUSE",
    description: "A card that adds a region map to the Pokégear.",
    consumable: false,
    tmhm_index: null,
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
    battle_menu: "ITEMMENU_NOUSE",
    description: "A card that enables Pokégear phone calls.",
    consumable: false,
    tmhm_index: null,
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
    battle_menu: "ITEMMENU_NOUSE",
    description: "A card that lets the Pokégear tune into radio stations.",
    consumable: false,
    tmhm_index: null,
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
    battle_menu: "ITEMMENU_NOUSE",
    description: "A card expanding the Pokégear radio with special programs.",
    consumable: false,
    tmhm_index: null,
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

function parseTmHmSymbols(content: string): Record<string, { script_name: string; tmhm_index: number }> {
  const symbols: Record<string, { script_name: string; tmhm_index: number }> = {};
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
      };
      hmNumber += 1;
      tmhmIndex += 1;
    }
  }
  return symbols;
}

function isUnusedAsmItemSlot(attributes: Record<string, string | number>): boolean {
  return (
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

export function exportItems(): ExportedItem[] {
  const root = getDisassemblyRoot();
  const attributesPath = path.join(root, "data", "items", "attributes.asm");
  const descriptionsPath = path.join(root, "data", "items", "descriptions.asm");
  const itemConstantsPath = path.join(root, "constants", "item_constants.asm");
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
  const tmhmSymbols = parseTmHmSymbols(fs.readFileSync(itemConstantsPath, "utf8"));
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
    items.push({
      name: String(attr.name),
      script_name: tmhmSymbol?.script_name ?? String(attr.source_name),
      effect: String(effect),
      price: Number(attr.price),
      held_effect: String(attr.held_effect),
      parameter: Number(attr.parameter),
      property: String(attr.property),
      pocket: String(attr.pocket),
      field_menu: String(attr.field_menu),
      battle_menu: String(attr.battle_menu),
      description,
      consumable: isConsumableItem(attr, tmhmIndex),
      tmhm_index: tmhmIndex,
    });
  }

  writeJsonToTargets("items.json", items, { indent: 2 });
  return items;
}
