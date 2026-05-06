import fs from "fs";
import path from "path";

export const ScriptInstructionType = {
  EMIT: "emit",
  WAIT_FRAMES: "wait_frames",
  WAIT_PROMPT: "wait_prompt",
  WAIT_DIALOGUE_CLEAR: "wait_dialogue_clear",
  WAIT_NOT_BUSY: "wait_not_busy",
  LABEL: "label",
  GOTO: "goto",
  CALL: "call",
  RETURN: "return",
  MACRO_CALL: "macro_call",
  LOOP: "loop",
} as const;

export type ScriptInstructionType =
  (typeof ScriptInstructionType)[keyof typeof ScriptInstructionType];

export class ScriptInstruction {
  public kind: ScriptInstructionType;
  public value: unknown;

  constructor(kind: ScriptInstructionType, value?: unknown) {
    this.kind = kind;
    this.value = value;
  }
}

export class ScriptProgram {
  public instructions: ScriptInstruction[];
  public labels: Record<string, number>;

  constructor(instructions: ScriptInstruction[], labels: Record<string, number>) {
    this.instructions = instructions;
    this.labels = labels;
  }
}

export class LoopInstruction {
  public program: ScriptProgram;
  public count: number;
  public rawBody: unknown[] | null;

  constructor(program: ScriptProgram, count: number, rawBody?: unknown[] | null) {
    this.program = program;
    this.count = count;
    this.rawBody = rawBody ?? null;
  }
}

export class MacroDefinition {
  public name: string;
  public params: string[];
  public body: unknown[];

  constructor(name: string, params: string[], body: unknown[]) {
    this.name = name;
    this.params = params;
    this.body = body;
  }

  expand(
    args: string[] | Record<string, unknown>,
    tokenizer: ScriptTokenizer,
    options?: { maxDepth?: number }
  ): ScriptProgram {
    const maxDepth = options?.maxDepth ?? 8;
    if (maxDepth <= 0) {
      throw new Error(`Macro expansion for '${this.name}' exceeded depth limit.`);
    }
    const mapping = this.buildMapping(args);
    const substituted = substitute(this.body, mapping);
    if (!Array.isArray(substituted)) {
      throw new Error(`Macro '${this.name}' expansion did not produce a script list.`);
    }
    return tokenizer.parse(substituted, { maxDepth: maxDepth - 1 });
  }

  private buildMapping(args: string[] | Record<string, unknown>): Record<string, string> {
    if (!Array.isArray(args)) {
      const mapping: Record<string, string> = {};
      for (const [key, value] of Object.entries(args)) {
        mapping[key] = String(value);
      }
      const missing = this.params.filter((param) => !(param in mapping));
      if (missing.length > 0) {
        throw new Error(`Macro '${this.name}' missing args: ${missing.join(", ")}`);
      }
      return mapping;
    }
    if (args.length !== this.params.length) {
      throw new Error(
        `Macro '${this.name}' expected ${this.params.length} args, got ${args.length}`
      );
    }
    const mapping: Record<string, string> = {};
    this.params.forEach((param, index) => {
      mapping[param] = String(args[index]);
    });
    return mapping;
  }
}

const substitute = (value: unknown, mapping: Record<string, string>): unknown => {
  if (typeof value === "string") {
    try {
      return value.replace(/\{([^}]+)\}/g, (_, key) => {
        if (!(key in mapping)) {
          throw new Error(`Missing macro arg for placeholder '${key}'`);
        }
        return mapping[key];
      });
    } catch (error) {
      if (error instanceof Error) {
        throw error;
      }
      throw new Error(String(error));
    }
  }
  if (Array.isArray(value)) {
    return value.map((item) => substitute(item, mapping));
  }
  if (isPlainObject(value)) {
    const output: Record<string, unknown> = {};
    for (const [key, val] of Object.entries(value)) {
      output[key] = substitute(val, mapping);
    }
    return output;
  }
  return value;
};

const isPlainObject = (value: unknown): value is Record<string, unknown> =>
  value !== null && typeof value === "object" && !Array.isArray(value);

const ensurePlainObject = (value: unknown, message: string): Record<string, unknown> => {
  if (!isPlainObject(value)) {
    throw new Error(message);
  }
  return value;
};

export class ScriptTokenizer {
  private static readonly MACRO_CALL_PATTERN = /^(?<name>[A-Za-z0-9_\-]+)\((?<args>.*)\)$/;
  private static readonly FIGHT_PATTERN = /^fight[:=]?(?<move>\d+)$/i;
  private static readonly SWITCH_PATTERN = /^switch[:=]?(?<slot>\d+)$/i;
  private static readonly RUN_PATTERN = /^run$/i;
  private static readonly ITEM_PATTERN =
    /^item[:=]?(?<pocket>[A-Za-z0-9_]+)[,/:](?<index>\d+)$/i;
  private static readonly WALKXY_PATTERN =
    /^walk\((?<x>-?\d+)\s*,\s*(?<y>-?\d+)\)$/i;
  private static readonly FACE_PATTERN = /^face[:=]?(?<dir>[A-Za-z]+)$/i;
  private static readonly USE_PATTERN = /^use[:=]?(?<what>[A-Za-z]+)$/i;
  private static readonly INTERACT_PATTERN = /^interact$/i;

  public baseDir: string | null;
  public aliases: Record<string, unknown[]> = {};
  public macros: Record<string, MacroDefinition> = {};

  constructor(options?: { baseDir?: string | null }) {
    this.baseDir = options?.baseDir ?? null;
  }

  parse(rawScript: Iterable<unknown>, options?: { maxDepth?: number }): ScriptProgram {
    const maxDepth = options?.maxDepth ?? 8;
    const instructions: ScriptInstruction[] = [];
    const labels: Record<string, number> = {};
    for (const entry of rawScript) {
      this.appendEntry(entry, instructions, labels, { depthRemaining: maxDepth });
    }
    return new ScriptProgram(instructions, labels);
  }

  private appendEntry(
    entry: unknown,
    instructions: ScriptInstruction[],
    labels: Record<string, number>,
    options: { depthRemaining: number }
  ): void {
    if (typeof entry === "string") {
      const expanded = this.expandAlias(entry);
      for (const subEntry of expanded) {
        if (typeof subEntry !== "string") {
          throw new Error(`Unsupported script entry: ${String(subEntry)}`);
        }
        this.appendScalar(subEntry, instructions, labels, options);
      }
      return;
    }
    if (isPlainObject(entry)) {
      this.appendMapping(entry, instructions, labels, options);
      return;
    }
    throw new Error(`Unsupported script entry: ${String(entry)}`);
  }

  private appendScalar(
    token: string,
    instructions: ScriptInstruction[],
    labels: Record<string, number>,
    options: { depthRemaining: number }
  ): void {
    const normalized = token.trim();
    if (!normalized) {
      return;
    }
    if (normalized.startsWith("label:")) {
      const label = normalized.slice("label:".length).trim();
      labels[label] = instructions.length;
      instructions.push(new ScriptInstruction(ScriptInstructionType.LABEL, label));
      return;
    }
    if (normalized.startsWith("goto:")) {
      const target = normalized.slice("goto:".length).trim();
      instructions.push(new ScriptInstruction(ScriptInstructionType.GOTO, target));
      return;
    }
    if (normalized.startsWith("call:")) {
      const target = normalized.slice("call:".length).trim();
      instructions.push(new ScriptInstruction(ScriptInstructionType.CALL, target));
      return;
    }
    if (normalized.startsWith("include:")) {
      const pathStr = normalized.slice("include:".length).trim();
      const included = this.loadInclude(pathStr);
      for (const subEntry of included) {
        this.appendEntry(subEntry, instructions, labels, {
          depthRemaining: options.depthRemaining - 1,
        });
      }
      return;
    }
    if (normalized === "return" || normalized === "ret") {
      instructions.push(new ScriptInstruction(ScriptInstructionType.RETURN));
      return;
    }
    if (normalized === "wait_prompt" || normalized === "wait-prompt") {
      instructions.push(new ScriptInstruction(ScriptInstructionType.WAIT_PROMPT));
      return;
    }
    if (
      normalized === "wait_dialogue" ||
      normalized === "wait_dialogue_clear" ||
      normalized === "wait-dialogue"
    ) {
      instructions.push(new ScriptInstruction(ScriptInstructionType.WAIT_DIALOGUE_CLEAR));
      return;
    }
    if (normalized === "wait_until_not_busy" || normalized === "wait-not-busy") {
      instructions.push(new ScriptInstruction(ScriptInstructionType.WAIT_NOT_BUSY));
      return;
    }
    const helperInstructions = this.maybeHelperInstruction(normalized);
    if (helperInstructions) {
      instructions.push(...helperInstructions);
      return;
    }
    if (normalized.startsWith("wait=") || normalized.startsWith("wait:")) {
      const frames = parseIntStrict(
        normalized.includes("=")
          ? normalized.split("=", 2)[1]
          : normalized.split(":", 2)[1]
      );
      instructions.push(new ScriptInstruction(ScriptInstructionType.WAIT_FRAMES, frames));
      return;
    }
    const macroCall = this.parseMacroInvocation(normalized);
    if (macroCall) {
      const [macroName, args] = macroCall;
      const macro = this.macros[macroName];
      if (!macro) {
        throw new Error(`Macro '${macroName}' is not defined.`);
      }
      instructions.push(
        new ScriptInstruction(ScriptInstructionType.MACRO_CALL, [
          macro,
          args,
          options.depthRemaining,
        ])
      );
      return;
    }
    const emitTokens = this.expandRepeat(normalized);
    instructions.push(new ScriptInstruction(ScriptInstructionType.EMIT, emitTokens));
  }

  private appendMapping(
    entry: Record<string, unknown>,
    instructions: ScriptInstruction[],
    labels: Record<string, number>,
    options: { depthRemaining: number }
  ): void {
    if (isCommentEntry(entry)) {
      return;
    }
    if ("include" in entry) {
      const included = this.loadInclude(String(entry.include));
      for (const subEntry of included) {
        this.appendEntry(subEntry, instructions, labels, {
          depthRemaining: options.depthRemaining - 1,
        });
      }
      return;
    }
    if ("aliases" in entry || "alias" in entry) {
      const mapping: Record<string, unknown> = {};
      if (isPlainObject(entry.aliases)) {
        Object.assign(mapping, entry.aliases);
      }
      if ("alias" in entry) {
        const alias = entry.alias;
        if (isPlainObject(alias)) {
          Object.assign(mapping, alias);
        } else if (alias !== undefined) {
          mapping[String(alias)] = [];
        }
      }
      for (const [key, rawValue] of Object.entries(mapping)) {
        let values: unknown[];
        if (Array.isArray(rawValue)) {
          values = rawValue;
        } else if (rawValue && typeof rawValue === "object") {
          values = [rawValue];
        } else {
          values = [String(rawValue)];
        }
        this.aliases[String(key)] = values;
      }
      return;
    }
    if ("macro" in entry) {
      const macroDef = ensurePlainObject(entry.macro, "Macro definition must be a mapping.");
      const name = String(macroDef.name ?? "").trim();
      const rawParams = macroDef.params;
      const params = Array.isArray(rawParams) ? rawParams.map((param) => String(param)) : [];
      const body = Array.isArray(macroDef.body) ? macroDef.body : [];
      if (!name) {
        throw new Error("Macro name is required.");
      }
      this.macros[name] = new MacroDefinition(name, params, body);
      return;
    }
    if ("loop" in entry) {
      const loopSpec = ensurePlainObject(entry.loop, "Loop entry must be a mapping with count and steps.");
      const count = parseIntStrict(loopSpec.count ?? 0);
      if (count < 0) {
        throw new Error("Loop count must be non-negative.");
      }
      const body = Array.isArray(loopSpec.steps) ? loopSpec.steps : [];
      const bodyProgram = this.parse(body, { maxDepth: options.depthRemaining - 1 });
      instructions.push(
        new ScriptInstruction(
          ScriptInstructionType.LOOP,
          new LoopInstruction(bodyProgram, count, [...body])
        )
      );
      return;
    }
    if ("walk" in entry) {
      const expanded = expandWalkEntry(entry.walk);
      instructions.push(new ScriptInstruction(ScriptInstructionType.EMIT, expanded));
      return;
    }
    if ("label" in entry) {
      const label = String(entry.label).trim();
      labels[label] = instructions.length;
      instructions.push(new ScriptInstruction(ScriptInstructionType.LABEL, label));
      return;
    }
    if ("goto" in entry) {
      const target = String(entry.goto).trim();
      instructions.push(new ScriptInstruction(ScriptInstructionType.GOTO, target));
      return;
    }
    if ("call" in entry) {
      const target = String(entry.call).trim();
      instructions.push(new ScriptInstruction(ScriptInstructionType.CALL, target));
      return;
    }
    if ("return" in entry) {
      instructions.push(new ScriptInstruction(ScriptInstructionType.RETURN));
      return;
    }
    if ("wait_prompt" in entry) {
      instructions.push(new ScriptInstruction(ScriptInstructionType.WAIT_PROMPT));
      return;
    }
    if ("wait_dialogue" in entry || "wait_dialogue_clear" in entry || "wait_until_not_busy" in entry) {
      const target = "wait_until_not_busy" in entry
        ? ScriptInstructionType.WAIT_NOT_BUSY
        : ScriptInstructionType.WAIT_DIALOGUE_CLEAR;
      instructions.push(new ScriptInstruction(target));
      return;
    }
    if ("wait" in entry) {
      const frames = parseIntStrict(entry.wait);
      instructions.push(new ScriptInstruction(ScriptInstructionType.WAIT_FRAMES, frames));
      return;
    }
    if ("macro_call" in entry) {
      const callSpec = ensurePlainObject(entry.macro_call, "macro_call must be a mapping.");
      const name = String(callSpec.name ?? "").trim();
      const macro = this.macros[name];
      if (!macro) {
        throw new Error(`Macro '${name}' is not defined.`);
      }
      const args = callSpec.args ?? [];
      instructions.push(
        new ScriptInstruction(ScriptInstructionType.MACRO_CALL, [
          macro,
          args,
          options.depthRemaining,
        ])
      );
      return;
    }
    const helperInstructions = this.maybeHelperMapping(entry, options.depthRemaining);
    if (helperInstructions) {
      instructions.push(...helperInstructions);
      return;
    }
    throw new Error(`Unknown script mapping: ${JSON.stringify(entry)}`);
  }

  private expandAlias(token: string): unknown[] {
    if (token in this.aliases) {
      return [...this.aliases[token]];
    }
    return [token];
  }

  private expandRepeat(token: string): string[] {
    for (const sep of ["x", "*"]) {
      if (token.includes(sep)) {
        const [prefix, suffix] = token.split(sep, 2);
        if (/^\d+$/.test(suffix)) {
          return Array(Math.max(1, parseIntStrict(suffix))).fill(prefix);
        }
      }
    }
    return [token];
  }

  private parseMacroInvocation(token: string): [string, string[]] | null {
    const match = ScriptTokenizer.MACRO_CALL_PATTERN.exec(token);
    if (!match || !match.groups) {
      return null;
    }
    const args = match.groups.args?.trim() ?? "";
    if (!args) {
      return [match.groups.name, []];
    }
    const parsedArgs = args
      .split(",")
      .map((arg) => arg.trim())
      .filter((arg) => arg.length > 0);
    return [match.groups.name, parsedArgs];
  }

  private loadInclude(pathStr: string): unknown[] {
    let includePath = pathStr;
    if (!path.isAbsolute(includePath) && this.baseDir) {
      includePath = path.resolve(this.baseDir, includePath);
    }
    if (!fs.existsSync(includePath)) {
      throw new Error(`Included script '${includePath}' not found.`);
    }
    const contents = fs.readFileSync(includePath, "utf-8");
    let parsed: unknown = null;
    try {
      parsed = JSON.parse(contents);
    } catch (error) {
      parsed = null;
    }
    if (Array.isArray(parsed)) {
      return parsed;
    }
    if (isPlainObject(parsed)) {
      const record = parsed;
      if (Array.isArray(record.script)) {
        return record.script;
      }
      return [record];
    }
    return contents
      .replace(/\n/g, " ")
      .split(" ")
      .filter((token) => token.length > 0);
  }

  private maybeHelperInstruction(normalized: string): ScriptInstruction[] | null {
    const fightMatch = ScriptTokenizer.FIGHT_PATTERN.exec(normalized);
    if (fightMatch?.groups?.move) {
      const moveIndex = parseIntStrict(fightMatch.groups.move);
      return helperToInstructions(battleFightSequence(moveIndex), { waitForPrompt: true });
    }
    if (ScriptTokenizer.RUN_PATTERN.test(normalized)) {
      return helperToInstructions(battleRunSequence(), { waitForPrompt: true });
    }
    const switchMatch = ScriptTokenizer.SWITCH_PATTERN.exec(normalized);
    if (switchMatch?.groups?.slot) {
      const slot = parseIntStrict(switchMatch.groups.slot);
      return helperToInstructions(battleSwitchSequence(slot), { waitForPrompt: true });
    }
    const itemMatch = ScriptTokenizer.ITEM_PATTERN.exec(normalized);
    if (itemMatch?.groups?.pocket && itemMatch?.groups?.index) {
      return helperToInstructions(
        battleItemSequence(itemMatch.groups.pocket, parseIntStrict(itemMatch.groups.index)),
        { waitForPrompt: true }
      );
    }
    const walkMatch = ScriptTokenizer.WALKXY_PATTERN.exec(normalized);
    if (walkMatch?.groups?.x && walkMatch?.groups?.y) {
      const dx = parseIntStrict(walkMatch.groups.x);
      const dy = parseIntStrict(walkMatch.groups.y);
      return helperToInstructions(walkXY(dx, dy));
    }
    const faceMatch = ScriptTokenizer.FACE_PATTERN.exec(normalized);
    if (faceMatch?.groups?.dir) {
      const direction = normalizeDirectionToken(faceMatch.groups.dir);
      return helperToInstructions([direction]);
    }
    if (ScriptTokenizer.INTERACT_PATTERN.test(normalized)) {
      return helperToInstructions(["a"], { waitForPrompt: true });
    }
    const useMatch = ScriptTokenizer.USE_PATTERN.exec(normalized);
    if (useMatch?.groups?.what) {
      // eslint-disable-next-line react-hooks/rules-of-hooks
      return helperToInstructions(useSequence(useMatch.groups.what), { waitForPrompt: true });
    }
    return null;
  }

  private maybeHelperMapping(
    entry: Record<string, unknown>,
    _depthRemaining: number
  ): ScriptInstruction[] | null {
    if ("fight" in entry) {
      return helperToInstructions(battleFightSequence(parseIntStrict(entry.fight)), {
        waitForPrompt: true,
      });
    }
    if ("run" in entry) {
      return helperToInstructions(battleRunSequence(), { waitForPrompt: true });
    }
    if ("switch" in entry) {
      return helperToInstructions(battleSwitchSequence(parseIntStrict(entry.switch)), {
        waitForPrompt: true,
      });
    }
    if ("item" in entry) {
      const itemSpec = entry.item;
      if (isPlainObject(itemSpec)) {
        const pocket = itemSpec.pocket ?? itemSpec.bag;
        const index = itemSpec.index ?? itemSpec.slot;
        if (pocket === undefined || index === undefined) {
          throw new Error("item helper requires pocket and index.");
        }
        return helperToInstructions(battleItemSequence(String(pocket), parseIntStrict(index)), {
          waitForPrompt: true,
        });
      }
      if (Array.isArray(itemSpec) && itemSpec.length === 2) {
        const [pocket, index] = itemSpec;
        return helperToInstructions(battleItemSequence(String(pocket), parseIntStrict(index)), {
          waitForPrompt: true,
        });
      }
      throw new Error("item helper requires [pocket, index] or mapping.");
    }
    if ("walk" in entry) {
      return helperToInstructions(expandWalkEntry(entry.walk));
    }
    if ("walk_xy" in entry) {
      const walkSpec = entry.walk_xy;
      if (isPlainObject(walkSpec)) {
        const dx = parseIntStrict(walkSpec.x ?? 0);
        const dy = parseIntStrict(walkSpec.y ?? 0);
        return helperToInstructions(walkXY(dx, dy));
      }
      if (Array.isArray(walkSpec) && walkSpec.length === 2) {
        return helperToInstructions(walkXY(parseIntStrict(walkSpec[0]), parseIntStrict(walkSpec[1])));
      }
      throw new Error("walk_xy helper requires x and y.");
    }
    if ("face" in entry) {
      const direction = normalizeDirectionToken(entry.face);
      return helperToInstructions([direction]);
    }
    if ("interact" in entry) {
      return helperToInstructions(["a"], { waitForPrompt: true });
    }
    if ("use" in entry) {
      // eslint-disable-next-line react-hooks/rules-of-hooks
      return helperToInstructions(useSequence(entry.use), { waitForPrompt: true });
    }
    return null;
  }
}

const helperToInstructions = (
  tokens: string[],
  options?: { waitForPrompt?: boolean }
): ScriptInstruction[] => {
  const instructions = [new ScriptInstruction(ScriptInstructionType.EMIT, tokens)];
  if (options?.waitForPrompt) {
    instructions.push(new ScriptInstruction(ScriptInstructionType.WAIT_PROMPT));
  }
  return instructions;
};

export const loadScriptSource = (filePath: string): unknown[] => {
  const rawText = fs.readFileSync(filePath, "utf-8");
  let parsed: unknown = null;
  try {
    parsed = JSON.parse(rawText);
  } catch (error) {
    parsed = null;
  }
  if (Array.isArray(parsed)) {
    return parsed;
  }
  if (isPlainObject(parsed)) {
    const record = parsed;
    if (Array.isArray(record.script)) {
      return record.script;
    }
    return [record];
  }
  return rawText
    .replace(/\n/g, " ")
    .split(" ")
    .filter((token) => token.length > 0);
};

const parseIntStrict = (value: unknown): number => {
  const parsed = Number.parseInt(String(value), 10);
  if (Number.isNaN(parsed)) {
    throw new Error(`Expected integer value for ${String(value)}`);
  }
  return parsed;
};

const normalizeDirectionToken = (direction: unknown): string => {
  const token = String(direction).trim().toLowerCase();
  const alias: Record<string, string> = { u: "up", d: "down", l: "left", r: "right" };
  const normalized = alias[token] ?? token;
  if (!["up", "down", "left", "right"].includes(normalized)) {
    throw new Error(`Unknown direction '${String(direction)}'.`);
  }
  return normalized;
};

const expandWalkEntry = (walk: unknown): string[] => {
  if (isPlainObject(walk)) {
    const mapping = walk;
    if ("x" in mapping || "y" in mapping) {
      const dx = parseIntStrict(mapping.x ?? 0);
      const dy = parseIntStrict(mapping.y ?? 0);
      return walkXY(dx, dy);
    }
    const expanded: string[] = [];
    for (const [direction, count] of Object.entries(mapping)) {
      expanded.push(...Array(Math.max(0, parseIntStrict(count))).fill(normalizeDirectionToken(direction)));
    }
    return expanded;
  }
  if (Array.isArray(walk)) {
    if (walk.length !== 2) {
      throw new Error("walk entry must provide [x, y] offsets.");
    }
    const dx = parseIntStrict(walk[0]);
    const dy = parseIntStrict(walk[1]);
    return walkXY(dx, dy);
  }
  throw new Error("walk entry must map directions or provide [x, y].");
};

const COMMENT_KEYS = new Set(["comment", "note", "description"]);

const isCommentEntry = (entry: Record<string, unknown>): boolean => {
  return Object.keys(entry).length > 0 && Object.keys(entry).every((key) => COMMENT_KEYS.has(key));
};

const battleMenuAnchor = (): string[] => ["up", "up", "left", "left"];
const moveMenuAnchor = (): string[] => ["up", "up", "left", "left"];

const battleFightSequence = (moveIndex: number): string[] => {
  if (moveIndex < 1 || moveIndex > 4) {
    throw new Error("fight helper expects move index 1-4.");
  }
  const row = Math.floor((moveIndex - 1) / 2);
  const col = (moveIndex - 1) % 2;
  const sequence: string[] = [];
  sequence.push(...battleMenuAnchor());
  sequence.push("a");
  sequence.push(...moveMenuAnchor());
  sequence.push(...Array(col).fill("right"));
  sequence.push(...Array(row).fill("down"));
  sequence.push("a");
  return sequence;
};

const battleRunSequence = (): string[] => {
  const sequence = battleMenuAnchor();
  sequence.push("down", "right", "a");
  return sequence;
};

const battleSwitchSequence = (slot: number): string[] => {
  if (slot < 1) {
    throw new Error("switch helper expects slot >= 1.");
  }
  const sequence = battleMenuAnchor();
  sequence.push("right", "a");
  sequence.push(...Array(6).fill("up"));
  sequence.push(...Array(slot - 1).fill("down"));
  sequence.push("a", "a");
  return sequence;
};

const battleItemSequence = (pocket: string, index: number): string[] => {
  if (index < 1) {
    throw new Error("item helper expects index >= 1.");
  }
  let normalized = pocket.toLowerCase();
  const pockets = ["item", "ball", "key", "tmhm"];
  if (normalized.startsWith("ball")) {
    normalized = "ball";
  }
  if (!pockets.includes(normalized)) {
    throw new Error(`Unknown pocket '${pocket}'. Expected one of: ${pockets.join(", ")}`);
  }
  const targetIdx = pockets.indexOf(normalized);
  const sequence = battleMenuAnchor();
  sequence.push("down", "a");
  sequence.push(...Array(3).fill("left"));
  sequence.push(...Array(targetIdx).fill("right"));
  sequence.push(...Array(8).fill("up"));
  sequence.push(...Array(index - 1).fill("down"));
  sequence.push("a");
  return sequence;
};

const walkXY = (dx: number, dy: number): string[] => {
  const sequence: string[] = [];
  if (dx > 0) {
    sequence.push(...Array(dx).fill("right"));
  } else if (dx < 0) {
    sequence.push(...Array(Math.abs(dx)).fill("left"));
  }
  if (dy > 0) {
    sequence.push(...Array(dy).fill("up"));
  } else if (dy < 0) {
    sequence.push(...Array(Math.abs(dy)).fill("down"));
  }
  return sequence;
};

const useSequence = (what: unknown): string[] => {
  const token = String(what).toLowerCase().trim();
  if (["bike", "rod", "old_rod", "good_rod", "super_rod"].includes(token)) {
    return ["select"];
  }
  if (["menu", "start"].includes(token)) {
    return ["start"];
  }
  throw new Error(`Unknown use target '${String(what)}'.`);
};
