/**
 * Parse move descriptions directly from the disassembly for faithful rendering.
 */
import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "./paths";
import { MoveName } from "./enums/move";
import { getAsmMoveNameOrder } from "./asm-move-names-loader";

type ParsedMoveDescriptions = {
  table: string[];
  descriptions: Record<string, string>;
};

const TABLE_LABEL = "MoveDescriptions::";
const TABLE_END_TOKEN = "assert_table_length";

const extractQuoted = (argument: string): string => {
  if (!argument.includes('"')) {
    return "";
  }
  const start = argument.indexOf('"');
  const end = argument.lastIndexOf('"');
  if (start === -1 || end <= start) {
    return "";
  }
  return argument.slice(start + 1, end).replace(/@+$/g, "");
};

const parseMoveDescriptionsFile = (filePath: string): ParsedMoveDescriptions => {
  const table: string[] = [];
  const descriptions: Record<string, string> = {};
  const lines = fs.readFileSync(filePath, "utf-8").split(/\r?\n/);
  let inTable = false;
  let pendingLabels: string[] = [];
  let buffer: string[] = [];

  const flush = (): void => {
    if (!pendingLabels.length) {
      buffer = [];
      return;
    }
    const text = buffer.join("").trim();
    for (const label of pendingLabels) {
      descriptions[label] = text;
    }
    pendingLabels = [];
    buffer = [];
  };

  for (const raw of lines) {
    const line = (raw.split(";", 1)[0] ?? "").trimEnd();
    const trimmed = line.trim();
    if (!trimmed) {
      continue;
    }
    if (trimmed === TABLE_LABEL) {
      inTable = true;
      continue;
    }
    if (inTable) {
      if (trimmed.startsWith(TABLE_END_TOKEN)) {
        inTable = false;
        continue;
      }
      if (trimmed.startsWith("dw ")) {
        const label = trimmed.slice(3).trim();
        if (label) {
          table.push(label);
        }
      }
      continue;
    }
    if (trimmed.endsWith(":")) {
      if (buffer.length) {
        flush();
      }
      pendingLabels.push(trimmed.replace(/:+$/, ""));
      continue;
    }
    if (!pendingLabels.length) {
      continue;
    }
    const firstSpaceIndex = trimmed.search(/\s/);
    const token = firstSpaceIndex === -1 ? trimmed : trimmed.slice(0, firstSpaceIndex);
    const argument = firstSpaceIndex === -1 ? "" : trimmed.slice(firstSpaceIndex).trim();
    if (token === "db") {
      const text = extractQuoted(argument);
      if (text) {
        buffer.push(text);
      }
      continue;
    }
    if (token === "next") {
      const text = extractQuoted(argument);
      if (text) {
        if (buffer.length) {
          buffer.push("\n");
        }
        buffer.push(text);
      }
    }
  }
  flush();
  return { table, descriptions };
};

export class AsmMoveDescriptionsLoader {
  private root: string;
  private cache: Record<MoveName, string> | null = null;

  constructor(disassemblyRoot?: string) {
    this.root = disassemblyRoot || getDisassemblyRoot();
  }

  public get(move: MoveName): string {
    const cache = this.cache ?? this.load();
    const description = cache[move];
    if (!description) {
      throw new Error(`Missing move description for ${move}`);
    }
    return description;
  }

  private load(): Record<MoveName, string> {
    const filePath = path.join(this.root, "data", "moves", "descriptions.asm");
    if (!fs.existsSync(filePath)) {
      throw new Error(`Move descriptions table not found at ${filePath}`);
    }
    const { table, descriptions } = parseMoveDescriptionsFile(filePath);
    const order = getAsmMoveNameOrder();
    if (table.length < order.length) {
      throw new Error(
        `Move description table length ${table.length} did not cover ${order.length} moves.`
      );
    }
    const mapping: Record<MoveName, string> = {} as Record<MoveName, string>;
    order.forEach((move, index) => {
      const label = table[index];
      const entry = descriptions[label];
      if (!entry) {
        throw new Error(`Move description missing for label ${label} (${move})`);
      }
      mapping[move] = entry;
    });
    this.cache = mapping;
    return mapping;
  }
}

export const asmMoveDescriptionsLoader = new AsmMoveDescriptionsLoader();
