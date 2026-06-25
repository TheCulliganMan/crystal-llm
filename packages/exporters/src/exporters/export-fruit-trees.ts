import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { writeJsonToTargets } from "./asm-utils";

export type ExportedFruitTreeCatalog = Record<string, string>;

const stripComment = (line: string): string => line.replace(/;.*/, "").trim();

const parseFruitTreeConstants = (content: string): string[] => {
  const constants: string[] = [];
  let inFruitTrees = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (line === "const_def 1") {
      inFruitTrees = true;
      continue;
    }
    if (!inFruitTrees) continue;
    const match = line.match(/^const\s+(FRUITTREE_[A-Z0-9_]+)$/);
    if (match) {
      constants.push(match[1]);
      continue;
    }
    if (constants.length > 0 && line.startsWith("DEF NUM_FRUIT_TREES")) {
      break;
    }
  }
  if (constants.length === 0) {
    throw new Error("No FRUITTREE_* constants found.");
  }
  return constants;
};

const parseFruitTreeItems = (content: string): string[] => {
  const items: string[] = [];
  let inFruitTreeItems = false;
  for (const rawLine of content.split(/\r?\n/)) {
    const line = stripComment(rawLine);
    if (line === "FruitTreeItems:") {
      inFruitTreeItems = true;
      continue;
    }
    if (!inFruitTreeItems || line.length === 0 || line.startsWith("table_width")) continue;
    if (line.startsWith("assert_table_length")) break;
    const match = line.match(/^db\s+([A-Z0-9_]+)$/);
    if (!match) {
      throw new Error(`Unsupported fruit tree item row '${line}'.`);
    }
    items.push(match[1]);
  }
  if (items.length === 0) {
    throw new Error("No FruitTreeItems entries found.");
  }
  return items;
};

export function exportFruitTrees(): ExportedFruitTreeCatalog {
  const root = getDisassemblyRoot();
  const constants = parseFruitTreeConstants(
    fs.readFileSync(path.join(root, "constants", "script_constants.asm"), "utf8")
  );
  const items = parseFruitTreeItems(fs.readFileSync(path.join(root, "data", "items", "fruit_trees.asm"), "utf8"));
  if (constants.length !== items.length) {
    throw new Error(`Fruit tree constant count ${constants.length} does not match item row count ${items.length}.`);
  }
  const catalog = Object.fromEntries(constants.map((constant, index) => [constant, items[index]]));
  writeJsonToTargets("fruit_trees.json", catalog, { indent: 2 });
  return catalog;
}
