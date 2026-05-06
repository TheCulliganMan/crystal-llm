import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { parseAsmNumber, writeJsonToTargets } from "./asm-utils";

export type ExportedItem = {
  name: string;
  price: number;
  held_effect: string;
  parameter: number;
  property: string;
  pocket: string;
  field_menu: string;
  battle_menu: string;
  description: string;
};

const ITEM_DESCRIPTION_ALIAS_SUFFIXES = [/(?:_|\s)(?:I|II|III|IV|V)$/i];

const SPECIAL_ITEM_OVERRIDES: Record<number, ExportedItem> = {
  0xfa: {
    name: "POKEGEAR",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    battle_menu: "ITEMMENU_NOUSE",
    description: "A versatile gadget that combines map and phone features.",
  },
  0xfb: {
    name: "MAP CARD",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    battle_menu: "ITEMMENU_NOUSE",
    description: "A card that adds a region map to the Pokégear.",
  },
  0xfc: {
    name: "PHONE CARD",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    battle_menu: "ITEMMENU_NOUSE",
    description: "A card that enables Pokégear phone calls.",
  },
  0xfd: {
    name: "RADIO CARD",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    battle_menu: "ITEMMENU_NOUSE",
    description: "A card that lets the Pokégear tune into radio stations.",
  },
  0xfe: {
    name: "EXPN CARD",
    price: 0,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT | CANT_TOSS",
    pocket: "KEY_ITEM",
    field_menu: "ITEMMENU_CLOSE",
    battle_menu: "ITEMMENU_NOUSE",
    description: "A card expanding the Pokégear radio with special programs.",
  },
  0xff: {
    name: "TM NIGHTMARE",
    price: 2000,
    held_effect: "HELD_NONE",
    parameter: 0,
    property: "CANT_SELECT",
    pocket: "TM_HM",
    field_menu: "ITEMMENU_PARTY",
    battle_menu: "ITEMMENU_NOUSE",
    description: "Teaches Nightmare to a compatible Pokémon.",
  },
};

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

function normalizeItemKey(name: string): string {
  return name.toUpperCase().replace(/[^A-Z0-9]/g, "");
}

function descriptionKeyCandidates(attributes: Record<string, string | number>): string[] {
  const candidates: string[] = [];
  for (const rawName of [attributes.source_name, attributes.name]) {
    if (typeof rawName !== "string" || !rawName.trim()) continue;
    const normalized = normalizeItemKey(rawName);
    if (normalized && !candidates.includes(normalized)) candidates.push(normalized);
    for (const pattern of ITEM_DESCRIPTION_ALIAS_SUFFIXES) {
      const alias = rawName.replace(pattern, "").trim();
      if (!alias) continue;
      const normalizedAlias = normalizeItemKey(alias);
      if (normalizedAlias && !candidates.includes(normalizedAlias)) candidates.push(normalizedAlias);
    }
  }
  return candidates;
}

export function exportItems(): ExportedItem[] {
  const root = getDisassemblyRoot();
  const attributesPath = path.join(root, "data", "items", "attributes.asm");
  const descriptionsPath = path.join(root, "data", "items", "descriptions.asm");
  const attributes = parseAttributes(fs.readFileSync(attributesPath, "utf8"));
  const descriptions = parseDescriptions(fs.readFileSync(descriptionsPath, "utf8"));
  const items: ExportedItem[] = [];

  for (let index = 0; index < 0x100; index += 1) {
    const override = SPECIAL_ITEM_OVERRIDES[index];
    if (override) {
      items.push(override);
      continue;
    }
    const attr = attributes[index];
    if (!attr) {
      items.push({
        name: `ITEM_${index.toString(16).toUpperCase().padStart(2, "0")}`,
        price: 0,
        held_effect: "HELD_NONE",
        parameter: 0,
        property: "0",
        pocket: "ITEM",
        field_menu: "ITEMMENU_NOUSE",
        battle_menu: "ITEMMENU_NOUSE",
        description: "No description available.",
      });
      continue;
    }
    let description = "No description available.";
    for (const candidate of descriptionKeyCandidates(attr)) {
      const descKey = Object.keys(descriptions).find((key) => normalizeItemKey(key.replace(/Desc$/, "")) === candidate);
      if (descKey) {
        description = descriptions[descKey] || description;
        break;
      }
    }
    items.push({
      name: String(attr.name),
      price: Number(attr.price),
      held_effect: String(attr.held_effect),
      parameter: Number(attr.parameter),
      property: String(attr.property),
      pocket: String(attr.pocket),
      field_menu: String(attr.field_menu),
      battle_menu: String(attr.battle_menu),
      description,
    });
  }

  writeJsonToTargets("items.json", items, { indent: 2 });
  return items;
}
