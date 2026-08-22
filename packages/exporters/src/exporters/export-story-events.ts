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

export type StandardScriptsPayload = {
  order: string[];
  scripts: StoryScripts;
  globalScriptRoots: string[];
};

export type GlobalScriptRootSource = {
  readonly filePath: string;
  readonly roots: readonly string[];
  readonly reachableLabels: readonly string[];
  readonly standardTargets: readonly string[];
};

export type SharedGlobalScriptRootSource = {
  readonly filePath: string;
  readonly roots: readonly string[];
};

export type GlobalScriptDefinitionSource = {
  readonly filePath: string;
  readonly roots: readonly string[];
  readonly reachableLabels: readonly string[];
};

export type StandardScriptsStoryEventPayload = {
  StandardScripts: Record<string, StoryCommand[] | string[]> & {
    StdScripts: StoryCommand[];
    GlobalScriptRoots: string[];
  };
};

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

const canonicalScriptArgs = (command: string, args: string[]): string[] => {
  if (command === "warpfacing") {
    if (args.length !== 4) {
      throw new Error(`warpfacing requires 4 args, found ${args.length}: ${args.join(", ")}`);
    }
    return [args[1], args[2], args[3], args[0]];
  }
  return args;
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

const isMovementDataLabel = (label: string): boolean =>
  label.startsWith("MovementData_") || /Movement\d*$/.test(label);
const isMapSectionLabel = (label: string): boolean => /_Map(?:Scripts|Events)$/.test(label);

const SCRIPT_TERMINATORS = new Set([
  "end",
  "endcallback",
  "farjumptext",
  "farsjump",
  "jump",
  "jumptext",
  "jumptextfaceplayer",
  "jumpstd",
  "memjump",
  "reloadend",
  "sjump",
  "stopandsjump",
  "endall",
  "halloffame",
  "credits",
  "return",
]);

// These commands do not return to the byte following the command. Their ASM
// handlers replace the script pointer with another script (`jp ScriptJump`),
// so a following source label is not an implicit fallthrough destination.
const SCRIPT_DYNAMIC_JUMP_TERMINATORS = new Set([
  "describedecoration",
  "fruittree",
  "scripttalkafter",
]);

export const isUnconditionalScriptTransferCommand = (command: string): boolean =>
  SCRIPT_TERMINATORS.has(command) || SCRIPT_DYNAMIC_JUMP_TERMINATORS.has(command);

const TEXT_COMMANDS = new Set([
  "text",
  "text_start",
  "text_block",
  "line",
  "para",
  "cont",
  "next",
  "done",
  "text_end",
  "prompt",
  "text_promptbutton",
  "text_ram",
  "text_decimal",
  "text_low",
  "text_pause",
  "text_today",
  "text_far",
  "sound_item",
  "sound_caught_mon",
  "sound_slot_machine_start",
  "sound_dex_fanfare_50_79",
  "sound_dex_fanfare_80_109",
  "sound_dex_fanfare_140_169",
  "sound_dex_fanfare_170_199",
  "sound_dex_fanfare_200_229",
  "sound_dex_fanfare_230_plus",
]);
const DATA_COMMANDS = new Set(["db", "dbw", "dw", "ds", "dn", "menu_coords"]);
// macros/scripts/maps.asm emits these as map-engine tables, not script-engine
// opcodes. Labels containing them are consumed by object/background/elevator/
// command-queue handlers and must never acquire a synthesized script edge.
const MAP_DATA_COMMANDS = new Set([
  "def_scene_scripts",
  "scene_const",
  "scene_script",
  "def_callbacks",
  "callback",
  "def_warp_events",
  "warp_event",
  "def_coord_events",
  "coord_event",
  "def_bg_events",
  "bg_event",
  "def_object_events",
  "object_event",
  "trainer",
  "itemball",
  "hiddenitem",
  "elevfloor",
  "conditional_event",
  "cmdqueue",
  "stonetable",
]);
const MOVEMENT_COMMANDS = new Set([
  "step",
  "slow_step",
  "big_step",
  "turn_step",
  "jump_step",
  "fast_jump_step",
  "slow_jump_step",
  "slide_step",
  "fast_slide_step",
  "slow_slide_step",
  "step_bump",
  "turn_head",
  "turn_away",
  "turn_in",
  "turn_waterfall",
  "step_sleep",
  "step_wait_end",
  "step_end",
  "step_loop",
  "step_stop",
  "fix_facing",
  "remove_fixed_facing",
  "set_sliding",
  "remove_sliding",
  "teleport_from",
  "teleport_to",
  "skyfall",
  "skyfall_top",
  "step_dig",
  "fish_got_bite",
  "fish_cast_rod",
  "hide_emote",
  "show_emote",
  "step_shake",
  "tree_shake",
  "rock_smash",
  "return_dig",
  "remove_object",
  "hide_object",
  "show_object",
]);
const CPU_COMMANDS = new Set([
  "add",
  "and",
  "bit",
  "call",
  "cp",
  "dec",
  "farcall",
  "farjp",
  "hlcoord",
  "inc",
  "jp",
  "jr",
  "ld",
  "ldh",
  "nop",
  "or",
  "pop",
  "push",
  "res",
  "ret",
  "rlca",
  "rrca",
  "rst",
  "scf",
  "set",
  "sla",
  "srl",
  "sub",
  "swap",
  "xor",
]);

const hasTextCommands = (commands: readonly StoryCommand[]): boolean =>
  commands.some((command) => TEXT_COMMANDS.has(command.command));
const hasDataCommands = (commands: readonly StoryCommand[]): boolean =>
  commands.some(
    (command) => DATA_COMMANDS.has(command.command) || MAP_DATA_COMMANDS.has(command.command),
  );
const hasOnlyMovementCommands = (commands: readonly StoryCommand[]): boolean =>
  commands.length > 0 && commands.every((command) => MOVEMENT_COMMANDS.has(command.command));
const hasCpuCommands = (commands: readonly StoryCommand[]): boolean =>
  commands.some((command) => CPU_COMMANDS.has(command.command));

export const isExecutableScriptControlTargetBody = (
  commands: readonly StoryCommand[],
): boolean => {
  const firstCommand = commands[0]?.command;
  return Boolean(
    firstCommand &&
      !TEXT_COMMANDS.has(firstCommand) &&
      !DATA_COMMANDS.has(firstCommand) &&
      !MAP_DATA_COMMANDS.has(firstCommand) &&
      !MOVEMENT_COMMANDS.has(firstCommand) &&
      !CPU_COMMANDS.has(firstCommand),
  );
};

export const isExecutableScriptControlEdge = (
  sourceCommands: readonly StoryCommand[],
  targetCommands: readonly StoryCommand[],
): boolean => {
  const sourceIsCpuRoutine = sourceCommands.some((command) => CPU_COMMANDS.has(command.command));
  if (sourceIsCpuRoutine) {
    const firstTargetCommand = targetCommands[0]?.command;
    return Boolean(firstTargetCommand && CPU_COMMANDS.has(firstTargetCommand));
  }
  return isExecutableScriptControlTargetBody(targetCommands);
};

const scriptCanFallThrough = (label: string, commands: readonly StoryCommand[]): boolean => {
  if (
    !commands.length ||
    isMapSectionLabel(label) ||
    isMovementDataLabel(label) ||
    hasOnlyMovementCommands(commands) ||
    hasDataCommands(commands) ||
    hasTextCommands(commands)
  ) {
    return false;
  }
  const last = commands.at(-1);
  if (!last) {
    return false;
  }
  if (isUnconditionalScriptTransferCommand(last.command)) {
    return false;
  }
  if (Array.isArray(last.args)) {
    // Unconditional CPU returns and jumps cannot reach the following source
    // label. Conditional forms (`ret z`, `jr nz, .target`, `jp c, .target`)
    // retain a real not-taken fallthrough and therefore still need the
    // explicit next-label edge.
    if (last.command === "ret" && last.args.length === 0) {
      return false;
    }
    if ((last.command === "jp" || last.command === "jr") && last.args.length === 1) {
      return false;
    }
    if (last.command === "farjp") {
      return false;
    }
  }
  return true;
};

const materializeScriptFallthroughs = (scripts: StoryScripts): void => {
  const entries = Object.entries(scripts);
  for (let index = 0; index < entries.length - 1; index += 1) {
    const [label, commands] = entries[index];
    if (!scriptCanFallThrough(label, commands)) {
      continue;
    }
    const [nextLabel, nextCommands] = entries[index + 1];
    if (
      !nextCommands.length ||
      isMovementDataLabel(nextLabel) ||
      hasOnlyMovementCommands(nextCommands) ||
      hasDataCommands(nextCommands) ||
      hasTextCommands(nextCommands)
    ) {
      continue;
    }
    if (hasCpuCommands(commands)) {
      // RGBDS labels inside CPU routines fall through in the instruction
      // stream. Materialize that implicit edge with a CPU jump so it remains
      // in the callasm/CPU interpreter domain; emitting `sjump` here would
      // falsely turn the next CPU label into script bytecode.
      commands.push({ command: "jp", args: [nextLabel] });
    } else {
      appendJumpIfMissing(scripts, label, nextLabel);
    }
  }
};

const materializeMovementFallthroughs = (scripts: StoryScripts): void => {
  const entries = Object.entries(scripts);
  for (let index = 0; index < entries.length; index += 1) {
    const [label, commands] = entries[index];
    if (!isMovementDataLabel(label) || commands.at(-1)?.command === "step_end") {
      continue;
    }
    for (let nextIndex = index + 1; nextIndex < entries.length; nextIndex += 1) {
      const [nextLabel, nextCommands] = entries[nextIndex];
      if (!isMovementDataLabel(nextLabel)) {
        break;
      }
      commands.push(...nextCommands.map((command) => ({ ...command })));
      if (commands.at(-1)?.command === "step_end") {
        break;
      }
    }
  }
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
      doors.forEach((doorParts, doorIndex) => {
        const doorNumber = doorIndex + 1;
        for (const bodyLine of body) {
          const bodyStripped = stripInlineComment(bodyLine).trim();
          if (bodyStripped === "changeugdoor n, OPEN") {
            for (const part of doorParts) {
              expanded.push(`\tchangeblock ${part.x}, ${part.y}, ${part.open}`);
            }
            continue;
          }
          if (bodyStripped === "changeugdoor n, CLOSED") {
            for (const part of doorParts) {
              expanded.push(`\tchangeblock ${part.x}, ${part.y}, ${part.closed}`);
            }
            continue;
          }
          expanded.push(
            bodyLine
              .replaceAll("{d:n}", String(doorNumber))
              .replace(/\bn\b/g, String(doorNumber))
          );
        }
      });
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

  const startLocalScript = (scriptName: string): void => {
    // A local label after a non-fallthrough command begins a distinct body.
    // Keep the parent name for RGBDS local-label scope, but stop copying the
    // local bytes into the parent command array.
    if (
      currentScriptName &&
      currentScript &&
      currentScript.length > 0 &&
      !scriptCanFallThrough(currentScriptName, currentScript)
    ) {
      currentScript = null;
    }
    const encoded = currentScriptName ? `${scriptName}@${currentScriptName}` : scriptName;
    const body = currentLocalScript?.length === 0 ? currentLocalScript : [];
    currentLocalScript = body;
    localScriptsData.set(encoded, body);
    if (currentScriptName) {
      localScriptsByParent.set(currentScriptName, [
        ...(localScriptsByParent.get(currentScriptName) ?? []),
        encoded,
      ]);
    } else {
      scripts[scriptName] = body;
    }
  };

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
        startLocalScript(scriptName);
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
      startLocalScript(localLabelMatch.groups.name);
      continue;
    }

    if (commandMatch) {
      const command = commandMatch[1];
      const argsSource = stripInlineComment(commandMatch[2]).trim();
      const commandDict: StoryCommand =
        command === "text" || command === "line" || command === "para" || command === "cont" || command === "done"
          ? { command, args: argsSource }
          : { command, args: canonicalScriptArgs(command, splitAsmArgs(argsSource)) };

      const activeScript: StoryCommand[] | null = currentScript;
      const activeLocalScript = currentLocalScript as StoryCommand[] | null;
      if (activeScript) {
        activeScript.push(commandDict);
      }
      if (activeLocalScript) {
        activeLocalScript.push(commandDict);
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
  materializeScriptFallthroughs(result);
  materializeMovementFallthroughs(result);
  applyKnownImplicitFallthroughs(result);
  return result;
}

const GLOBAL_SCRIPT_DIRECT_CONTROL_TARGETS = new Set([
  "iftrue",
  "iffalse",
  "sjump",
  "jump",
  "farsjump",
  "scall",
  "farscall",
  "sdefer",
]);
const GLOBAL_SCRIPT_COMPARE_CONTROL_TARGETS = new Set([
  "ifequal",
  "ifnotequal",
  "ifgreater",
  "ifless",
]);
const GLOBAL_SCRIPT_LOCAL_BODY_TARGET_INDEX = new Map<string, number>([
  ["writetext", 0],
  ["applymovement", 1],
]);

const declaredFarScriptLabels = (filePath: string): Map<string, number> => {
  const declarations = new Map<string, number>();
  for (const rawLine of fs.readFileSync(filePath, "utf8").split(/\r?\n/)) {
    const line = stripInlineComment(rawLine).trim();
    const match = line.match(/^([A-Za-z_.][A-Za-z0-9_]*)::$/);
    if (!match) {
      continue;
    }
    declarations.set(match[1], (declarations.get(match[1]) ?? 0) + 1);
  }
  return declarations;
};

const resolveGlobalSourceLabel = (
  scripts: StoryScripts,
  sourceLabel: string,
  target: string,
): string | undefined => {
  if (scripts[target]) {
    return target;
  }
  if (!target.startsWith(".")) {
    return undefined;
  }
  const parent = sourceLabel.includes("@")
    ? sourceLabel.slice(sourceLabel.lastIndexOf("@") + 1)
    : sourceLabel;
  const scoped = `${target}@${parent}`;
  return scripts[scoped] ? scoped : undefined;
};

const requiredGlobalControlTarget = (command: StoryCommand): string | undefined => {
  if (!Array.isArray(command.args)) {
    return undefined;
  }
  if (GLOBAL_SCRIPT_DIRECT_CONTROL_TARGETS.has(command.command)) {
    return command.args[0];
  }
  if (GLOBAL_SCRIPT_COMPARE_CONTROL_TARGETS.has(command.command)) {
    return command.args[1];
  }
  return undefined;
};

const collectGlobalScriptRootClosure = (
  source: GlobalScriptRootSource,
  standardOrder: readonly string[],
  standardAndSharedScripts: StoryScripts,
  requireFarRootDeclarations = true,
): { scripts: StoryScripts; standardTargets: string[] } => {
  if (!fs.existsSync(source.filePath)) {
    throw new Error(`Required global script source is missing: ${source.filePath}`);
  }
  if (source.roots.length === 0) {
    throw new Error(`Global script source ${source.filePath} has no required roots`);
  }
  const duplicateRoot = source.roots.find(
    (root, index) => source.roots.indexOf(root) !== index,
  );
  if (duplicateRoot) {
    throw new Error(`Global script root ${duplicateRoot} is declared more than once`);
  }
  const duplicateStandardTarget = source.standardTargets.find(
    (target, index) => source.standardTargets.indexOf(target) !== index,
  );
  if (duplicateStandardTarget) {
    throw new Error(
      `Global script source ${source.filePath} repeats standard target ${duplicateStandardTarget}`,
    );
  }
  const duplicateReachableLabel = source.reachableLabels.find(
    (label, index) => source.reachableLabels.indexOf(label) !== index,
  );
  if (duplicateReachableLabel) {
    throw new Error(
      `Global script source ${source.filePath} repeats reachable label ${duplicateReachableLabel}`,
    );
  }

  const declarations = declaredFarScriptLabels(source.filePath);
  const sourceScripts = parseAsmFile(source.filePath);
  for (const root of source.roots) {
    if (requireFarRootDeclarations && declarations.get(root) !== 1) {
      throw new Error(
        `Required global script root ${root} must be declared with :: in ${source.filePath}`,
      );
    }
    if (!sourceScripts[root]?.length) {
      throw new Error(`Required global script root ${root} has no command body in ${source.filePath}`);
    }
  }
  for (const target of source.standardTargets) {
    if (!standardOrder.includes(target)) {
      throw new Error(
        `Required global script standard target ${target} is not declared by the exact StdScripts pointer table`,
      );
    }
    if (!standardAndSharedScripts[target]?.length) {
      throw new Error(`Required global script standard target ${target} has no command body`);
    }
  }

  const reachable = new Set<string>();
  const actualStandardTargets: string[] = [];
  const visit = (label: string): void => {
    if (reachable.has(label)) {
      return;
    }
    const body = sourceScripts[label];
    if (!body?.length) {
      throw new Error(`Global script ${label} has no command body in ${source.filePath}`);
    }
    reachable.add(label);
    for (const [commandIndex, command] of body.entries()) {
      if (command.command === "jumpstd") {
        const target = Array.isArray(command.args) ? command.args[0] : undefined;
        if (!target || !standardOrder.includes(target) || !standardAndSharedScripts[target]?.length) {
          throw new Error(
            `Global script ${label} command ${commandIndex} jumpstd targets missing exact standard script ${String(target)}`,
          );
        }
        actualStandardTargets.push(target);
        continue;
      }

      const requiredTarget = requiredGlobalControlTarget(command);
      if (requiredTarget) {
        const localTarget = resolveGlobalSourceLabel(sourceScripts, label, requiredTarget);
        if (localTarget) {
          visit(localTarget);
        } else if (!standardAndSharedScripts[requiredTarget]?.length) {
          throw new Error(
            `Global script ${label} command ${commandIndex} ${command.command} targets missing script ${requiredTarget}`,
          );
        }
      }

      const localBodyTargetIndex = GLOBAL_SCRIPT_LOCAL_BODY_TARGET_INDEX.get(command.command);
      if (localBodyTargetIndex !== undefined) {
        const target = Array.isArray(command.args)
          ? command.args[localBodyTargetIndex]
          : undefined;
        const localTarget = target
          ? resolveGlobalSourceLabel(sourceScripts, label, target)
          : undefined;
        if (!target || !localTarget) {
          throw new Error(
            `Global script ${label} command ${commandIndex} ${command.command} targets missing local body ${String(target)}`,
          );
        }
        visit(localTarget);
      }

      if (Array.isArray(command.args)) {
        for (const argument of command.args) {
          const localTarget = resolveGlobalSourceLabel(sourceScripts, label, argument);
          if (localTarget) {
            visit(localTarget);
          }
        }
      }
    }
  };
  for (const root of source.roots) {
    visit(root);
  }

  const reachableLabels = Object.keys(sourceScripts).filter((label) => reachable.has(label));
  if (
    reachableLabels.length !== source.reachableLabels.length ||
    reachableLabels.some((label, index) => label !== source.reachableLabels[index])
  ) {
    throw new Error(
      `Global script source ${source.filePath} must expose exactly reachable labels [${source.reachableLabels.join(
        ", ",
      )}], found [${reachableLabels.join(", ")}]`,
    );
  }

  if (
    actualStandardTargets.length !== source.standardTargets.length ||
    actualStandardTargets.some((target, index) => target !== source.standardTargets[index])
  ) {
    throw new Error(
      `Global script source ${source.filePath} must target exactly [${source.standardTargets.join(
        ", ",
      )}] through jumpstd, found [${actualStandardTargets.join(", ")}]`,
    );
  }

  return {
    scripts: Object.fromEntries(
      Object.entries(sourceScripts).filter(([label]) => reachable.has(label)),
    ),
    standardTargets: actualStandardTargets,
  };
};

export function parseStandardScriptsFile(
  filePath: string,
  sharedScriptFilePaths: readonly string[] = [],
  globalScriptSources: readonly GlobalScriptRootSource[] = [],
  sharedGlobalScriptSources: readonly SharedGlobalScriptRootSource[] = [],
  globalScriptDefinitionSources: readonly GlobalScriptDefinitionSource[] = [],
): StandardScriptsPayload {
  const source = fs.readFileSync(filePath, "utf8");
  const order = source
    .split(/\r?\n/)
    .map((rawLine) => stripInlineComment(rawLine).trim())
    .flatMap((line) => {
      const match = line.match(/^add_stdscript\s+([A-Za-z_][A-Za-z0-9_]*)$/);
      return match ? [match[1]] : [];
    });
  if (order.length === 0) {
    throw new Error(`Standard script pointer table in ${filePath} is empty`);
  }
  const duplicate = order.find((label, index) => order.indexOf(label) !== index);
  if (duplicate) {
    throw new Error(`Standard script pointer table in ${filePath} repeats ${duplicate}`);
  }

  const parsed = parseAsmFile(filePath);
  const scripts: StoryScripts = Object.fromEntries(
    Object.entries(parsed).filter(([label]) => label !== "StdScripts"),
  );
  const definitionSources = new Map(
    Object.keys(scripts).map((label) => [label, filePath] as const),
  );
  for (const sharedScriptFilePath of sharedScriptFilePaths) {
    if (!fs.existsSync(sharedScriptFilePath)) {
      throw new Error(`Required shared script source is missing: ${sharedScriptFilePath}`);
    }
    for (const [label, commands] of Object.entries(
      parseAsmFile(sharedScriptFilePath),
    )) {
      if (Object.hasOwn(scripts, label)) {
        throw new Error(
          `Shared script label ${label} from ${sharedScriptFilePath} duplicates ${filePath}`,
        );
      }
      scripts[label] = commands;
      definitionSources.set(label, sharedScriptFilePath);
    }
  }
  for (const label of order) {
    if (!scripts[label]?.length) {
      throw new Error(`Standard script pointer ${label} has no parsed command body in ${filePath}`);
    }
  }

  const globalScriptRoots: string[] = [];
  const declaredGlobalRoots = new Set<string>();
  const declaredSharedGlobalSources = new Set<string>();
  for (const source of sharedGlobalScriptSources) {
    if (!sharedScriptFilePaths.includes(source.filePath)) {
      throw new Error(
        `Shared global script source ${source.filePath} is not a canonical shared script source`,
      );
    }
    if (declaredSharedGlobalSources.has(source.filePath)) {
      throw new Error(`Shared global script source ${source.filePath} is declared more than once`);
    }
    declaredSharedGlobalSources.add(source.filePath);
    if (source.roots.length === 0) {
      throw new Error(`Shared global script source ${source.filePath} has no required roots`);
    }
    for (const root of source.roots) {
      if (declaredGlobalRoots.has(root)) {
        throw new Error(`Global script root ${root} is declared more than once`);
      }
      declaredGlobalRoots.add(root);
      const definitionSource = definitionSources.get(root);
      if (definitionSource !== source.filePath) {
        throw new Error(
          `Required shared global script root ${root} is not defined by ${source.filePath}`,
        );
      }
      if (!scripts[root]?.length) {
        throw new Error(
          `Required shared global script root ${root} has no command body in ${source.filePath}`,
        );
      }
      globalScriptRoots.push(root);
    }
  }
  for (const source of globalScriptDefinitionSources) {
    const closure = collectGlobalScriptRootClosure(
      { ...source, standardTargets: [] },
      order,
      scripts,
      false,
    );
    for (const [label, commands] of Object.entries(closure.scripts)) {
      const previousSource = definitionSources.get(label);
      if (previousSource) {
        throw new Error(
          `Global script definition ${label} from ${source.filePath} duplicates ${previousSource}`,
        );
      }
      if (label === "StdScripts" || label === "GlobalScriptRoots") {
        throw new Error(`Global script definition source ${source.filePath} uses reserved label ${label}`);
      }
      scripts[label] = commands;
      definitionSources.set(label, source.filePath);
    }
  }
  for (const source of globalScriptSources) {
    for (const root of source.roots) {
      if (declaredGlobalRoots.has(root)) {
        throw new Error(`Global script root ${root} is declared more than once`);
      }
      declaredGlobalRoots.add(root);
    }
    const closure = collectGlobalScriptRootClosure(source, order, scripts);
    for (const [label, commands] of Object.entries(closure.scripts)) {
      const previousSource = definitionSources.get(label);
      if (previousSource) {
        throw new Error(
          `Global script label ${label} from ${source.filePath} duplicates ${previousSource}`,
        );
      }
      if (label === "StdScripts" || label === "GlobalScriptRoots") {
        throw new Error(`Global script source ${source.filePath} uses reserved label ${label}`);
      }
      scripts[label] = commands;
      definitionSources.set(label, source.filePath);
    }
    globalScriptRoots.push(...source.roots);
  }
  return { order, scripts, globalScriptRoots };
}

export function standardScriptsStoryEventPayload(
  standardScripts: StandardScriptsPayload,
): StandardScriptsStoryEventPayload {
  return {
    StandardScripts: {
      StdScripts: standardScripts.order.map((label) => ({
        command: "add_stdscript",
        args: [label],
      })),
      GlobalScriptRoots: standardScripts.globalScriptRoots,
      ...standardScripts.scripts,
    },
  };
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

  const standardScriptsPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "events",
    "std_scripts.asm",
  );
  if (!fs.existsSync(standardScriptsPath)) {
    throw new Error(`Required standard script source is missing: ${standardScriptsPath}`);
  }
  const sharedOverworldScriptsPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "events",
    "overworld.asm",
  );
  const sharedTreeMonScriptsPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "events",
    "treemons.asm",
  );
  const sharedSweetScentScriptsPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "events",
    "sweet_scent.asm",
  );
  const sharedMiscScriptsPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "events",
    "misc_scripts.asm",
  );
  const sharedTreeMonMapsPath = path.join(
    getDisassemblyRoot(),
    "data",
    "wild",
    "treemon_maps.asm",
  );
  const sharedTreeMonsPath = path.join(
    getDisassemblyRoot(),
    "data",
    "wild",
    "treemons.asm",
  );
  const bugContestScriptsPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "events",
    "bug_contest",
    "contest.asm",
  );
  const clearSpritesPath = path.join(
    getDisassemblyRoot(),
    "home",
    "clear_sprites.asm",
  );
  const fieldMovesPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "events",
    "field_moves.asm",
  );
  const fishingGfxPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "events",
    "fishing_gfx.asm",
  );
  const mapSetupPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "overworld",
    "map_setup.asm",
  );
  const standardScripts = parseStandardScriptsFile(
    standardScriptsPath,
    [
      sharedOverworldScriptsPath,
      sharedTreeMonScriptsPath,
      sharedSweetScentScriptsPath,
      sharedMiscScriptsPath,
      sharedTreeMonMapsPath,
      sharedTreeMonsPath,
    ],
    [
      {
        filePath: bugContestScriptsPath,
        roots: [
          "BugCatchingContestBattleScript",
          "BugCatchingContestOverScript",
        ],
        reachableLabels: [
          "BugCatchingContestBattleScript",
          "BugCatchingContestOverScript",
          "BugCatchingContestOutOfBallsScript",
          "BugCatchingContestReturnToGateScript",
          "BugCatchingContestTimeUpText",
          "BugCatchingContestIsOverText",
        ],
        standardTargets: ["BugContestResultsWarpScript"],
      },
    ],
    [
      {
        filePath: sharedOverworldScriptsPath,
        roots: [
          "Script_CutFromMenu",
          "Script_UseFlash",
          "SurfFromMenuScript",
          ".FlyScript@FlyFunction",
          "Script_WaterfallFromMenu",
          ".UsedDigScript@EscapeRopeOrDig",
          ".UsedEscapeRopeScript@EscapeRopeOrDig",
          ".TeleportScript@TeleportFunction",
          "Script_StrengthFromMenu",
          "Script_WhirlpoolFromMenu",
          "HeadbuttFromMenuScript",
          "RockSmashFromMenuScript",
          "Script_GotABite",
          "Script_NotEvenANibble",
          "Script_NotEvenANibble2",
          "Script_GetOnBike",
          "Script_GetOnBike_Register",
          "Script_GetOffBike",
          "Script_GetOffBike_Register",
          "Script_CantGetOffBike",
        ],
      },
      {
        filePath: sharedSweetScentScriptsPath,
        roots: [".SweetScent@SweetScentFromMenu"],
      },
    ],
    [
      {
        filePath: clearSpritesPath,
        roots: ["HideSprites"],
        reachableLabels: ["HideSprites", ".loop@HideSprites"],
      },
      {
        filePath: fieldMovesPath,
        roots: ["BlindingFlash", "ShakeHeadbuttTree", "FlyFromAnim", "FlyToAnim"],
        reachableLabels: [
          "BlindingFlash",
          "ShakeHeadbuttTree",
          ".loop@ShakeHeadbuttTree",
          ".done@ShakeHeadbuttTree",
          "HideHeadbuttTree",
          "TreeRelativeLocationTable",
          "OWCutAnimation",
          ".loop@OWCutAnimation",
          ".finish@OWCutAnimation",
          ".LoadCutGFX@OWCutAnimation",
          "OWCutJumptable",
          ".dw@OWCutJumptable",
          "Cut_SpawnAnimateTree",
          "Cut_SpawnAnimateLeaves",
          "Cut_StartWaiting",
          "Cut_WaitAnimSFX",
          ".finished@Cut_WaitAnimSFX",
          "Cut_SpawnLeaf",
          "Cut_GetLeafSpawnCoords",
          ".left_side@Cut_GetLeafSpawnCoords",
          ".top_side@Cut_GetLeafSpawnCoords",
          ".Coords@Cut_GetLeafSpawnCoords",
          "Cut_Headbutt_GetPixelFacing",
          ".Coords@Cut_Headbutt_GetPixelFacing",
          "FlyFromAnim",
          ".loop@FlyFromAnim",
          ".exit@FlyFromAnim",
          "FlyToAnim",
          ".loop@FlyToAnim",
          ".exit@FlyToAnim",
          ".RestorePlayerSprite_DespawnLeaves@FlyToAnim",
          ".OAMloop@FlyToAnim",
          "FlyFunction_InitGFX",
          "FlyFunction_FrameTimer",
          ".exit@FlyFunction_FrameTimer",
          ".SpawnLeaf@FlyFunction_FrameTimer",
        ],
      },
      {
        filePath: fishingGfxPath,
        roots: ["LoadFishingGFX"],
        reachableLabels: [
          "LoadFishingGFX",
          ".got_gender@LoadFishingGFX",
          ".LoadGFX@LoadFishingGFX",
        ],
      },
      {
        filePath: mapSetupPath,
        roots: ["SkipUpdateMapSprites"],
        reachableLabels: ["SkipUpdateMapSprites"],
      },
    ],
  );
  writeJsonToTargets(
    path.join("story_events", "StandardScripts.json"),
    standardScriptsStoryEventPayload(standardScripts),
    { indent: 2 },
  );

  const overworldEventsPath = path.join(
    getDisassemblyRoot(),
    "engine",
    "overworld",
    "events.asm",
  );
  if (!fs.existsSync(overworldEventsPath)) {
    throw new Error(`Required overworld event source is missing: ${overworldEventsPath}`);
  }
  const overworldEvents = parseAsmFile(overworldEventsPath);
  for (const sharedPath of [
    path.join(getDisassemblyRoot(), "engine", "events", "trainer_scripts.asm"),
    path.join(getDisassemblyRoot(), "engine", "events", "whiteout.asm"),
    path.join(getDisassemblyRoot(), "engine", "pokemon", "breeding.asm"),
    path.join(getDisassemblyRoot(), "engine", "overworld", "player_object.asm"),
  ]) {
    if (!fs.existsSync(sharedPath)) {
      throw new Error(`Required player-event source is missing: ${sharedPath}`);
    }
    for (const [label, commands] of Object.entries(parseAsmFile(sharedPath))) {
      if (overworldEvents[label]) {
        throw new Error(
          `Player-event label ${label} from ${sharedPath} duplicates ${overworldEventsPath}`,
        );
      }
      overworldEvents[label] = commands;
    }
  }
  writeJsonToTargets(
    path.join("story_events", "OverworldEvents.json"),
    { OverworldEvents: overworldEvents },
    { indent: 2 },
  );
}
