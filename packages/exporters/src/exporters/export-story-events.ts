import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { getTypeScriptDataDir, stripInlineComment, splitAsmArgs, toSnakeCase, writeJsonToTargets } from "./asm-utils";

const LABEL_RE = /^(?<name>[a-zA-Z_.][a-zA-Z0-9_]*)(?<suffix>::?|:)\s*(?:;.*)?$/;
const LOCAL_LABEL_RE = /^(?<name>\.[a-zA-Z0-9_]+)\s*(?:;.*)?$/;
const COMMAND_RE = /^\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(.*)$/;
const ACTIVE_DEFINES = new Set<string>();

export type StoryCommand = {
  command: string;
  args: string[] | string;
};

export type StoryScripts = Record<string, StoryCommand[]>;

type UndergroundDoorPart = {
  readonly x: string;
  readonly y: string;
  readonly closed: string;
  readonly open: string;
};

type ConditionalState = {
  readonly parentActive: boolean;
  readonly conditionMatched: boolean;
  active: boolean;
};

const isConditionalStackActive = (stack: readonly ConditionalState[]): boolean =>
  stack.every((state) => state.active);

const evaluateAsmCondition = (line: string): boolean => {
  const defMatch = line.match(/^if\s+DEF\(([^)]+)\)$/);
  if (defMatch) {
    return ACTIVE_DEFINES.has(defMatch[1].trim());
  }
  return true;
};

const applyConditionalDirective = (
  line: string,
  stack: ConditionalState[]
): boolean => {
  if (line.startsWith("if ")) {
    const parentActive = isConditionalStackActive(stack);
    const conditionMatched = evaluateAsmCondition(line);
    stack.push({
      parentActive,
      conditionMatched,
      active: parentActive && conditionMatched,
    });
    return true;
  }
  if (line === "else") {
    const current = stack.at(-1);
    if (!current) {
      throw new Error("Encountered ASM else without a matching if.");
    }
    current.active = current.parentActive && !current.conditionMatched;
    return true;
  }
  if (line === "endc") {
    if (!stack.pop()) {
      throw new Error("Encountered ASM endc without a matching if.");
    }
    return true;
  }
  return false;
};

const appendJumpIfMissing = (
  scripts: StoryScripts,
  scriptName: string,
  targetScriptName: string
): void => {
  const script = scripts[scriptName];
  if (!script?.length) {
    return;
  }
  const last = script[script.length - 1];
  const command = String(last?.command ?? "").toLowerCase();
  if (command === "sjump" || command === "jump") {
    return;
  }
  script.push({ command: "sjump", args: [targetScriptName] });
};

const applyKnownImplicitFallthroughs = (scripts: StoryScripts): void => {
  appendJumpIfMissing(
    scripts,
    "AzaleaTownRivalBattleScene2",
    "AzaleaTownRivalBattleScript"
  );
};

const parseUgDoorDefs = (lines: readonly string[]): UndergroundDoorPart[][] => {
  const doors: UndergroundDoorPart[][] = [];
  for (const rawLine of lines) {
    const line = stripInlineComment(rawLine).trim();
    if (!line.startsWith("ugdoor_def ")) {
      continue;
    }
    const args = splitAsmArgs(line.slice("ugdoor_def ".length));
    if (args.length % 4 !== 0) {
      throw new Error(`Malformed ugdoor_def: ${line}`);
    }
    const parts: UndergroundDoorPart[] = [];
    for (let index = 0; index < args.length; index += 4) {
      parts.push({
        x: args[index],
        y: args[index + 1],
        closed: args[index + 2],
        open: args[index + 3],
      });
    }
    doors.push(parts);
  }
  return doors;
};

const expandGoldenrodSwitchDoorMacroScripts = (
  filePath: string,
  lines: readonly string[]
): string[] => {
  if (path.basename(filePath) !== "GoldenrodUndergroundSwitchRoomEntrances.asm") {
    return [...lines];
  }
  const doors = parseUgDoorDefs(lines);
  if (!doors.length) {
    return [...lines];
  }

  const expanded: string[] = [];
  for (let index = 0; index < lines.length; index += 1) {
    const rawLine = lines[index];
    const stripped = stripInlineComment(rawLine).trim();
    const loopMatch = stripped.match(/^for\s+n,\s*1,\s*ugdoor_n\s*\+\s*1$/);
    if (!loopMatch) {
      expanded.push(rawLine);
      continue;
    }

    const body: string[] = [];
    index += 1;
    while (index < lines.length) {
      const loopLine = lines[index];
      if (stripInlineComment(loopLine).trim() === "endr") {
        break;
      }
      body.push(loopLine);
      index += 1;
    }

    const labelLine = body.find((line) => stripInlineComment(line).trim().startsWith(".OpenDoor{d:n}:")
      || stripInlineComment(line).trim().startsWith(".CloseDoor{d:n}:"));
    const state = labelLine?.includes(".OpenDoor") ? "OPEN" : labelLine?.includes(".CloseDoor") ? "CLOSED" : null;
    if (!state) {
      expanded.push(rawLine, ...body, "endr");
      continue;
    }
    const eventCommand = state === "OPEN" ? "setevent" : "clearevent";
    const blockKey = state === "OPEN" ? "open" : "closed";
    const labelPrefix = state === "OPEN" ? ".OpenDoor" : ".CloseDoor";

    doors.forEach((doorParts, doorIndex) => {
      const doorNumber = doorIndex + 1;
      expanded.push(`${labelPrefix}${doorNumber}:`);
      for (const part of doorParts) {
        expanded.push(`\tchangeblock ${part.x}, ${part.y}, ${part[blockKey]}`);
      }
      expanded.push(`\t${eventCommand} EVENT_DOOR_${doorNumber}_OPEN`);
      expanded.push("\tend");
    });
  }
  return expanded;
};

export function parseAsmFile(filePath: string): StoryScripts {
  const sourceLines = fs.readFileSync(filePath, "utf8").split(/\r?\n/);
  const lines = expandGoldenrodSwitchDoorMacroScripts(filePath, sourceLines);
  const scripts: StoryScripts = {};
  let currentScriptName: string | null = null;
  let currentScript: StoryCommand[] | null = null;
  let currentLocalScript: StoryCommand[] | null = null;
  const parentScriptsOrdered: string[] = [];
  const localScriptsByParent = new Map<string, string[]>();
  const localScriptsData = new Map<string, StoryCommand[]>();
  const conditionalStack: ConditionalState[] = [];

  for (const rawLine of lines) {
    const lineStripped = rawLine.trim();
    const line = rawLine.replace(/\r$/, "");

    if (!lineStripped || lineStripped.startsWith(";")) {
      continue;
    }

    const lineWithoutComment = stripInlineComment(lineStripped).trim();
    if (applyConditionalDirective(lineWithoutComment, conditionalStack)) {
      continue;
    }

    if (!isConditionalStackActive(conditionalStack)) {
      continue;
    }

    const labelMatch = line.match(LABEL_RE);
    const localLabelMatch = line.match(LOCAL_LABEL_RE);
    const commandMatch = line[0]?.match(/\s/) ? line.match(COMMAND_RE) : null;

    if (labelMatch?.groups?.name) {
      const scriptName = labelMatch.groups.name;
      if (scriptName.startsWith(".")) {
        if (currentScriptName) {
          currentLocalScript = [];
          const encoded = `${scriptName}@${currentScriptName}`;
          localScriptsData.set(encoded, currentLocalScript);
          localScriptsByParent.set(currentScriptName, [
            ...(localScriptsByParent.get(currentScriptName) ?? []),
            encoded,
          ]);
        } else {
          currentLocalScript = [];
          scripts[scriptName] = currentLocalScript;
        }
      } else {
        currentScriptName = scriptName;
        currentScript = [];
        scripts[currentScriptName] = currentScript;
        parentScriptsOrdered.push(currentScriptName);
        currentLocalScript = null;
      }
      continue;
    }

    if (localLabelMatch?.groups?.name) {
      const scriptName = localLabelMatch.groups.name;
      const encoded = currentScriptName ? `${scriptName}@${currentScriptName}` : scriptName;
      currentLocalScript = [];
      localScriptsData.set(encoded, currentLocalScript);
      if (currentScriptName) {
        localScriptsByParent.set(currentScriptName, [
          ...(localScriptsByParent.get(currentScriptName) ?? []),
          encoded,
        ]);
      }
      continue;
    }

    if (commandMatch) {
      const command = commandMatch[1];
      const argsSource = stripInlineComment(commandMatch[2]).trim();
      const commandDict: StoryCommand =
        command === "text" || command === "line" || command === "para" || command === "cont" || command === "done"
          ? { command, args: argsSource }
          : { command, args: splitAsmArgs(argsSource) };

      if (currentScript) {
        currentScript.push(commandDict);
      }
      if (currentLocalScript) {
        currentLocalScript.push(commandDict);
      }
      continue;
    }

    if (currentScript || currentLocalScript) {
      if (currentScriptName && currentScript && currentScript.length === 0) {
        delete scripts[currentScriptName];
      }
      currentScript = null;
      currentScriptName = null;
      currentLocalScript = null;
    }
  }

  const orderedScripts: StoryScripts = {};
  for (const parent of parentScriptsOrdered) {
    if (scripts[parent]) {
      orderedScripts[parent] = scripts[parent];
      for (const local of localScriptsByParent.get(parent) ?? []) {
        const localData = localScriptsData.get(local);
        if (localData) {
          orderedScripts[local] = localData;
        }
      }
    }
  }

  for (const [key, value] of Object.entries(scripts)) {
    if (!orderedScripts[key]) {
      orderedScripts[key] = value;
    }
  }
  for (const [key, value] of localScriptsData.entries()) {
    if (!orderedScripts[key]) {
      orderedScripts[key] = value;
    }
  }

  const result = Object.fromEntries(
    Object.entries(orderedScripts).filter(([, value]) => value.length > 0)
  );
  applyKnownImplicitFallthroughs(result);
  return result;
}

export function exportStoryEvents(): void {
  const asmMapsDir = path.join(getDisassemblyRoot(), "maps");
  const storyEventsDir = path.join(getTypeScriptDataDir(), "story_events");
  const mapOutputsDir = path.join(getTypeScriptDataDir(), "maps");
  const aggregate: Record<string, StoryScripts> = {};

  for (const asmFile of fs.readdirSync(asmMapsDir).filter((entry) => entry.endsWith(".asm")).sort()) {
    const sourcePath = path.join(asmMapsDir, asmFile);
    const scripts = parseAsmFile(sourcePath);
    if (Object.keys(scripts).length === 0) {
      continue;
    }

    const stem = path.basename(asmFile, ".asm");
    aggregate[stem] = scripts;
    const snakeName = `${toSnakeCase(stem)}.json`;
    const storyTargetPath = path.join(storyEventsDir, snakeName);
    const relativeTarget = fs.existsSync(storyTargetPath)
      ? path.join("story_events", snakeName)
      : path.join("maps", `${stem}.json`);

    writeJsonToTargets(relativeTarget, scripts, { indent: 2 });
  }

  writeJsonToTargets("story_events.json", aggregate, { indent: 2 });
}
