import fs from "fs";
import path from "path";

export const RUNTIME_PRESENTATION_ENTRYPOINTS = [
  "boot",
  "intro",
  "title",
  "main_menu",
  "continue",
  "new_game",
  "delete_save",
  "reset_clock",
] as const;

type RuntimePresentationEntrypoint =
  (typeof RUNTIME_PRESENTATION_ENTRYPOINTS)[number];

export type RuntimePresentationSourceSpan = {
  file: string;
  start_line: number;
  end_line: number;
};

export type RuntimePresentationOperation = {
  op: string;
  source_span: RuntimePresentationSourceSpan;
  [key: string]: unknown;
};

type RuntimePresentationHostEffectCallForm =
  "call" | "callfar" | "farcall" | "jump" | "restart";

type RuntimePresentationHostEffectWrite =
  | {
      kind: "copy_bytes";
      source: string;
      target: string;
      byte_count: number;
      fields: string[];
      values?: string[];
      source_span: RuntimePresentationSourceSpan;
    }
  | {
      kind: "copy_byte";
      source: string;
      target: string;
      source_span: RuntimePresentationSourceSpan;
    }
  | {
      kind: "constant_byte";
      target: string;
      value: number;
      source_span: RuntimePresentationSourceSpan;
    }
  | {
      kind: "zero_bytes";
      targets: string[];
      source_span: RuntimePresentationSourceSpan;
    }
  | {
      kind: "persist_rtc";
      sources: string[];
      targets: string[];
      clears_halt: boolean;
      source_span: RuntimePresentationSourceSpan;
    };

export type RuntimePresentationHostEffect = {
  id: string;
  call_target: string;
  accepted_call_forms: RuntimePresentationHostEffectCallForm[];
  result: {
    name: string;
    type: "enum";
    domain: Array<{
      id: string;
      value: number;
      conditions: Array<{ source: string; valid: boolean }>;
    }>;
  };
  validity_checks: Array<{
    source: string;
    fields: Array<{ name: string; equals: number }>;
    source_span: RuntimePresentationSourceSpan;
  }>;
  state_deltas: Array<{
    when: string;
    writes: RuntimePresentationHostEffectWrite[];
  }>;
  required_consumer: { id: string; required: true };
  source_span: RuntimePresentationSourceSpan;
  implementation_source_spans: RuntimePresentationSourceSpan[];
};

export type RuntimePresentationProgram = {
  schema_version: 1;
  entrypoints: Record<RuntimePresentationEntrypoint, string>;
  blocks: Record<
    string,
    {
      source_span: RuntimePresentationSourceSpan;
      operations: RuntimePresentationOperation[];
    }
  >;
  resources: Array<{
    path: string;
    kind: "tiles" | "tilemap" | "attrmap" | "palette";
    source_span: RuntimePresentationSourceSpan;
  }>;
  audio: Array<{
    id: string;
    kind: "music" | "sound_effect" | "cry" | "silence";
    source_span: RuntimePresentationSourceSpan;
  }>;
  text: Array<{
    id: string;
    source_span: RuntimePresentationSourceSpan;
  }>;
  host_effects: RuntimePresentationHostEffect[];
  subprograms: RuntimePresentationCallableSubprogram[];
};

export type BuildRuntimeTitlePresentationProgramOptions = {
  disassemblyRoot: string;
  audioAssetIds: ReadonlySet<string>;
  runtimeSpawnIdentifiers: ReadonlySet<number>;
  readSource?: (relativePath: string) => string;
};

type LoadedSource = {
  file: string;
  lines: string[];
};

export type RuntimePresentationAsmInstruction = {
  opcode: string;
  args: string[];
  source_span: RuntimePresentationSourceSpan;
};

export type RuntimePresentationControlFlow = {
  entrypoints: Record<RuntimePresentationEntrypoint, string>;
  blocks: Record<
    string,
    {
      source_span: RuntimePresentationSourceSpan;
      instructions: RuntimePresentationAsmInstruction[];
      direct_targets: string[];
      fallthrough: string | null;
    }
  >;
  indirect_tables: Array<{
    source_span: RuntimePresentationSourceSpan;
    table: string;
    entries: string[];
    index_domain: { minimum: number; maximum: number; values: number[] } | null;
  }>;
  external_calls: Array<{
    target: string;
    call_form: "call" | "callfar" | "farcall" | "jump" | "restart";
    args: [];
    source_span: RuntimePresentationSourceSpan;
  }>;
  sprite_operations: RuntimePresentationSpriteOperation[];
  sprite_programs: RuntimePresentationSpriteProgram[];
  sprite_diagnostics: Array<{
    table: string;
    message: string;
    source_span: RuntimePresentationSourceSpan;
  }>;
};

type RuntimePresentationNamedByte = {
  symbol: string;
  value: number;
};

export type RuntimePresentationSpriteOperation =
  | {
      op: "sprite_init";
      instance: string;
      object: RuntimePresentationNamedByte;
      source_span: RuntimePresentationSourceSpan;
      allocation_source_span: RuntimePresentationSourceSpan;
    }
  | {
      op: "sprite_scheduler_step";
      instances: string[];
      source_span: RuntimePresentationSourceSpan;
      before_host_call: string;
    };

type RuntimePresentationSpriteFrameset = RuntimePresentationNamedByte & {
  table_source_span: RuntimePresentationSourceSpan;
  data_source_span: RuntimePresentationSourceSpan;
  frames: Array<{
    oam_set: RuntimePresentationNamedByte;
    duration: number;
    flags: string | null;
    source_span: RuntimePresentationSourceSpan;
  }>;
  waits: Array<{
    duration: number;
    source_span: RuntimePresentationSourceSpan;
    implementation_source_span: RuntimePresentationSourceSpan;
  }>;
  terminal: {
    op: "end" | "restart" | "delete";
    source_span: RuntimePresentationSourceSpan;
  };
};

type RuntimePresentationOuterMemoryRead = {
  source_symbol: string;
  symbol: string;
  predicate: "nonzero" | "equals";
  comparison_value: number | null;
  source_span: RuntimePresentationSourceSpan;
  alias_source_spans: RuntimePresentationSourceSpan[];
};

type RuntimePresentationOuterByteDomain = {
  symbol: string;
  initialized_value: number;
  minimum: number;
  maximum: number;
  values: number[];
  initializer_source_span: RuntimePresentationSourceSpan;
};

export type RuntimePresentationSpriteProgram = {
  instance: string;
  struct_slot: number;
  initializer_source_span: RuntimePresentationSourceSpan;
  allocation_source_span: RuntimePresentationSourceSpan;
  object: RuntimePresentationNamedByte & {
    table_source_span: RuntimePresentationSourceSpan;
  };
  initial_memory: {
    index: number;
    frameset_id: number;
    anim_seq_id: number;
    tile_id: number;
    xcoord: number;
    ycoord: number;
    xoffset: number;
    yoffset: number;
    duration: number;
    duration_offset: number;
    frame: number;
    jumptable_index: number;
    var1: number;
    var2: number;
    var3: number;
    var4: number;
  };
  frameset: RuntimePresentationSpriteFrameset;
  frameset_variants: RuntimePresentationSpriteFrameset[];
  callback: RuntimePresentationNamedByte &
    (
      | {
          kind: "state_table";
          table_source_span: RuntimePresentationSourceSpan;
          wrapper: string;
          wrapper_source_span: RuntimePresentationSourceSpan;
          target: string;
          target_source_span: RuntimePresentationSourceSpan;
          state_table: {
            table: string;
            source_span: RuntimePresentationSourceSpan;
            entries: Array<{
              index: number;
              target: string;
              source_span: RuntimePresentationSourceSpan;
              instructions: RuntimePresentationAsmInstruction[];
            }>;
            index_domain: {
              minimum: number;
              maximum: number;
              values: number[];
            };
          };
        }
      | {
          kind: "direct";
          table_source_span: RuntimePresentationSourceSpan;
          target: string;
          target_source_span: RuntimePresentationSourceSpan;
          instructions: RuntimePresentationAsmInstruction[];
          per_tick_struct_deltas: Partial<
            Record<
              keyof RuntimePresentationSpriteProgram["initial_memory"],
              number
            >
          >;
          host_operations: Array<{
            op: "sine" | "cosine";
            target: string;
            inputs: string[];
            output: string;
            source_span: RuntimePresentationSourceSpan;
            wrapper_source_span: RuntimePresentationSourceSpan;
            implementation_source_span: RuntimePresentationSourceSpan;
          }>;
          outer_memory_reads: RuntimePresentationOuterMemoryRead[];
          frameset_reinitializations: Array<{
            frameset: RuntimePresentationNamedByte;
            guard: RuntimePresentationOuterMemoryRead | null;
            application: "every_reachable_scheduler_tick";
            source_span: RuntimePresentationSourceSpan;
            implementation_source_span: RuntimePresentationSourceSpan;
            reachable_scheduler_ticks: number[];
          }>;
          struct_control_byte_domains: Array<{
            property: keyof RuntimePresentationSpriteProgram["initial_memory"];
            initialized_value: number;
            minimum: number;
            maximum: number;
            values: number[];
          }>;
        }
    );
  callback_data_resources: Array<{
    symbol: string;
    kind: "rgb555_palette";
    path: string;
    include_source_span: RuntimePresentationSourceSpan;
    data_source_span: RuntimePresentationSourceSpan;
    bytes: number[];
    colors: Array<{
      red: number;
      green: number;
      blue: number;
      source_span: RuntimePresentationSourceSpan;
    }>;
  }>;
  dictionary: RuntimePresentationNamedByte;
  oam_resources: Array<{
    oam_set: RuntimePresentationNamedByte;
    tile_offset: number;
    data_target: string;
    table_source_span: RuntimePresentationSourceSpan;
    data_source_span: RuntimePresentationSourceSpan;
    sprites: RuntimePresentationAsmInstruction[];
  }>;
  outer_state_effects: Array<{
    from_callback_index: number;
    to_callback_index: number;
    symbol: string;
    operation: "increment";
    source_span: RuntimePresentationSourceSpan;
    helper_source_span: RuntimePresentationSourceSpan;
  }>;
  lifetime: {
    allocation_dispatcher_entry: number | null;
    allocation_dispatch_tick: number;
    active_dispatcher_entries: number[];
    scheduler_ticks: number | null;
    deinitialized_after_dispatch_tick: number | null;
    callback_before_frame_update: true;
    outer_byte_domains: RuntimePresentationOuterByteDomain[];
    deinitializer: {
      op: "deinitialize_all_sprites" | "clear_sprite_anims";
      dispatcher_entry: number;
      dispatch_tick: number;
      before_scheduler_step: true;
      source_span: RuntimePresentationSourceSpan;
      implementation_source_span: RuntimePresentationSourceSpan;
      reachable_dispatch_ticks: number[];
    } | null;
    outer_scene_advances: Array<{
      dispatcher_entry: number;
      dispatch_tick: number;
      source_span: RuntimePresentationSourceSpan;
    }>;
    pre_scheduler_waits: Array<{
      target: "DelayFrames";
      dispatcher_entry: number;
      dispatch_tick: number;
      frame_count: number;
      before_scheduler_step: true;
      source_span: RuntimePresentationSourceSpan;
      implementation_source_span: RuntimePresentationSourceSpan;
    }>;
    handler_host_operations: Array<{
      target: string;
      dispatcher_entry: number;
      source_span: RuntimePresentationSourceSpan;
      dispatch_ticks: number[];
    }>;
  };
};

export type RuntimePresentationCallableSubprogram = {
  id: string;
  source_entry: string;
  accepted_call_forms: RuntimePresentationHostEffectCallForm[];
  result: {
    name: string;
    storage: "carry";
    domain: Array<{
      id: string;
      value: number | null;
      condition: Record<string, unknown>;
      source_span: RuntimePresentationSourceSpan;
    }>;
  };
  phases: Array<{
    id: string;
    source_span: RuntimePresentationSourceSpan;
    operations: RuntimePresentationOperation[];
  }>;
  loop: {
    source_span: RuntimePresentationSourceSpan;
    order: string[];
    input: Record<string, unknown>;
    scene_dispatch: Record<string, unknown>;
    natural_scheduler_ticks: number;
    scheduler: RuntimePresentationSpriteOperation;
    frame_wait: RuntimePresentationOperation;
  };
  resource_transfers: Array<Record<string, unknown>>;
  tilemap_writes: Array<Record<string, unknown>>;
  resources: Array<{
    path: string;
    kind: "tiles" | "tilemap" | "attrmap" | "palette";
    include_source_span: RuntimePresentationSourceSpan;
    data_source_span: RuntimePresentationSourceSpan;
  }>;
  audio: Array<{
    id: string;
    kind: "music" | "sound_effect" | "cry" | "silence";
    source_span: RuntimePresentationSourceSpan;
  }>;
  sprite_operations: RuntimePresentationSpriteOperation[];
  sprite_programs: RuntimePresentationSpriteProgram[];
  required_consumer: { id: string; required: true };
  source_span: RuntimePresentationSourceSpan;
  implementation_source_spans: RuntimePresentationSourceSpan[];
};

const PROGRAM_SOURCE_FILES = [
  "home/init.asm",
  "engine/menus/intro_menu.asm",
  "engine/menus/main_menu.asm",
  "engine/menus/delete_save.asm",
  "engine/rtc/reset_password.asm",
  "engine/rtc/restart_clock.asm",
  "engine/rtc/timeset.asm",
  "engine/menus/init_gender.asm",
  "engine/movie/title.asm",
  "engine/movie/splash.asm",
  "engine/movie/intro.asm",
] as const;

const SPRITE_SOURCE_FILES = [
  "home/delay.asm",
  "home/sprite_anims.asm",
  "engine/sprite_anims/core.asm",
  "engine/sprite_anims/functions.asm",
  "data/sprite_anims/objects.asm",
  "data/sprite_anims/framesets.asm",
  "data/sprite_anims/oam.asm",
] as const;

const CONSTANT_SOURCE_FILES = [
  "constants/misc_constants.asm",
  "constants/ram_constants.asm",
  "constants/sprite_anim_constants.asm",
  "constants/gfx_constants.asm",
  "constants/hardware.inc",
] as const;

const TYPED_SPRITE_BOUNDARIES = new Set([
  "InitSpriteAnimStruct",
  "PlaySpriteAnimations",
]);

const PRESENTATION_ENTRYPOINT_LABELS: Record<
  RuntimePresentationEntrypoint,
  string
> = {
  boot: "GameInit",
  intro: "IntroSequence",
  title: "StartTitleScreen",
  main_menu: "Intro_MainMenu",
  continue: "Continue",
  new_game: "NewGame",
  delete_save: "DeleteSaveData",
  reset_clock: "ResetClock",
};

type ParsedAsmBlock = {
  id: string;
  globalLabel: string;
  file: string;
  startLine: number;
  instructions: RuntimePresentationAsmInstruction[];
  nextBlock: string | null;
};

const splitInstructionArgs = (value: string): string[] => {
  const result: string[] = [];
  let current = "";
  let inQuotes = false;
  let parentheses = 0;
  for (const char of value) {
    if (char === '"') inQuotes = !inQuotes;
    if (!inQuotes && char === "(") parentheses += 1;
    if (!inQuotes && char === ")") parentheses -= 1;
    if (!inQuotes && parentheses === 0 && char === ",") {
      result.push(current.trim());
      current = "";
    } else {
      current += char;
    }
  }
  if (current.trim()) result.push(current.trim());
  return result;
};

const parseInstruction = (
  source: LoadedSource,
  lineIndex: number,
  value: string,
): RuntimePresentationAsmInstruction => {
  const [opcode = "", ...tail] = value.split(/\s+/);
  return {
    opcode,
    args: splitInstructionArgs(tail.join(" ")),
    source_span: {
      file: source.file,
      start_line: lineIndex + 1,
      end_line: lineIndex + 1,
    },
  };
};

const parseAsmBlocks = (
  sources: LoadedSource[],
): Map<string, ParsedAsmBlock> => {
  const blocks = new Map<string, ParsedAsmBlock>();
  for (const source of sources) {
    let globalLabel = "";
    let current: ParsedAsmBlock | null = null;
    const fileBlocks: ParsedAsmBlock[] = [];
    for (let lineIndex = 0; lineIndex < source.lines.length; lineIndex += 1) {
      const normalized = normalizeAsmLine(source.lines[lineIndex]);
      if (!normalized) continue;
      const explicitLabel = normalized.match(
        /^([A-Za-z_.][A-Za-z0-9_.@]*|[+-])::?\s*(.*)$/,
      );
      const implicitLocalLabel = normalized.match(
        /^(\.[A-Za-z_][A-Za-z0-9_.@]*|[+-])$/,
      );
      const label =
        explicitLabel ??
        (implicitLocalLabel
          ? [implicitLocalLabel[0], implicitLocalLabel[1], ""]
          : null);
      const isLabel = !!label;
      if (isLabel && label) {
        const rawLabel = label[1];
        let id: string;
        if (rawLabel.startsWith(".")) {
          if (!globalLabel) {
            throw new Error(
              `Local presentation label ${rawLabel} at ${source.file}:${lineIndex + 1} has no global owner`,
            );
          }
          id = `${rawLabel}@${globalLabel}`;
        } else if (rawLabel === "+" || rawLabel === "-") {
          if (!globalLabel) {
            throw new Error(
              `Anonymous presentation label ${rawLabel} at ${source.file}:${lineIndex + 1} has no global owner`,
            );
          }
          id = `${rawLabel}@${globalLabel}@${lineIndex + 1}`;
        } else {
          globalLabel = rawLabel;
          id = rawLabel;
        }
        if (blocks.has(id)) {
          throw new Error(`Duplicate runtime presentation label ${id}`);
        }
        current = {
          id,
          globalLabel,
          file: source.file,
          startLine: lineIndex + 1,
          instructions: [],
          nextBlock: null,
        };
        blocks.set(id, current);
        fileBlocks.push(current);
        if (label[2]) {
          current.instructions.push(
            parseInstruction(source, lineIndex, label[2]),
          );
        }
        continue;
      }
      if (current) {
        current.instructions.push(
          parseInstruction(source, lineIndex, normalized),
        );
      }
    }
    for (let index = 0; index + 1 < fileBlocks.length; index += 1) {
      fileBlocks[index].nextBlock = fileBlocks[index + 1].id;
    }
  }
  return blocks;
};

const resolveControlTarget = (
  sourceBlock: ParsedAsmBlock,
  rawTarget: string,
  blocks?: Map<string, ParsedAsmBlock>,
  sourceLine?: number,
): string =>
  rawTarget.startsWith(".") && !rawTarget.includes("@")
    ? `${rawTarget}@${sourceBlock.globalLabel}`
    : /^[+-]+$/.test(rawTarget) && blocks && sourceLine
      ? (() => {
          const direction = rawTarget[0];
          const ordinal = rawTarget.length;
          const candidates = [...blocks.values()]
            .filter(
              (candidate) =>
                candidate.file === sourceBlock.file &&
                candidate.globalLabel === sourceBlock.globalLabel &&
                candidate.id.startsWith(`${direction}@`) &&
                (direction === "+"
                  ? candidate.startLine > sourceLine
                  : candidate.startLine < sourceLine),
            )
            .sort((left, right) =>
              direction === "+"
                ? left.startLine - right.startLine
                : right.startLine - left.startLine,
            );
          const resolved = candidates[ordinal - 1];
          if (!resolved) {
            throw new Error(
              `Anonymous runtime presentation target ${rawTarget} at ${sourceBlock.file}:${sourceLine} has no exact ${ordinal === 1 ? "" : `${ordinal}th `}${direction === "+" ? "next" : "previous"} label`,
            );
          }
          return resolved.id;
        })()
      : rawTarget;

const instructionTarget = (
  block: ParsedAsmBlock,
  instruction: RuntimePresentationAsmInstruction,
  blocks: Map<string, ParsedAsmBlock>,
): string | null => {
  if (
    !["call", "callfar", "farcall", "jp", "jr"].includes(instruction.opcode)
  ) {
    return null;
  }
  const rawTarget = instruction.args.at(-1);
  return rawTarget
    ? resolveControlTarget(
        block,
        rawTarget,
        blocks,
        instruction.source_span.start_line,
      )
    : null;
};

const isConditionalControl = (
  instruction: RuntimePresentationAsmInstruction,
): boolean =>
  ["jp", "jr", "ret"].includes(instruction.opcode) &&
  instruction.args.length > (instruction.opcode === "ret" ? 0 : 1);

const isUnconditionalTerminal = (
  instruction: RuntimePresentationAsmInstruction | undefined,
): boolean =>
  !!instruction &&
  ((["jp", "jr"].includes(instruction.opcode) &&
    instruction.args.length === 1) ||
    (instruction.opcode === "ret" && instruction.args.length === 0));

const tableEntries = (
  blocks: Map<string, ParsedAsmBlock>,
  tableId: string,
): string[] => {
  const table = blocks.get(tableId);
  if (!table) return [];
  const entries: string[] = [];
  let cursor: ParsedAsmBlock | undefined = table;
  while (cursor) {
    for (const instruction of cursor.instructions) {
      if (instruction.opcode !== "dw") {
        return entries;
      }
      for (const raw of instruction.args) {
        entries.push(
          resolveControlTarget(
            cursor,
            raw,
            blocks,
            instruction.source_span.start_line,
          ),
        );
      }
    }
    if (!cursor.nextBlock?.startsWith(".")) break;
    const next = blocks.get(cursor.nextBlock);
    if (
      !next?.instructions.every((instruction) => instruction.opcode === "dw")
    ) {
      break;
    }
    cursor = next;
  }
  return entries;
};

const precedingTableSymbol = (
  block: ParsedAsmBlock,
  beforeIndex: number,
): string | null => {
  for (let index = beforeIndex - 1; index >= 0; index -= 1) {
    const instruction = block.instructions[index];
    if (
      (instruction.opcode === "ld" || instruction.opcode === "ldh") &&
      instruction.args.length === 2 &&
      instruction.args[0] === "hl"
    ) {
      return resolveControlTarget(block, instruction.args[1]);
    }
  }
  return null;
};

type ByteDomain = Set<number>;
type AbstractFlagSource =
  { kind: "a_zero" } | { kind: "a_compare"; value: number } | null;

type AbstractByteState = {
  block: string;
  instruction: number;
  a: ByteDomain;
  flagSource: AbstractFlagSource;
};

const allByteValues = (): ByteDomain =>
  new Set(Array.from({ length: 256 }, (_, value) => value));

const singletonByte = (value: number): ByteDomain =>
  new Set([((value % 256) + 256) % 256]);

const sortedDomain = (domain: ReadonlySet<number>): number[] =>
  [...domain].sort((left, right) => left - right);

const domainKey = (state: AbstractByteState): string =>
  `${state.block}:${state.instruction}:${sortedDomain(state.a).join(",")}:${
    state.flagSource?.kind ?? "unknown"
  }:${state.flagSource?.kind === "a_compare" ? state.flagSource.value : ""}`;

const conditionDomains = (
  state: AbstractByteState,
  condition: string,
): { taken: ByteDomain; fallthrough: ByteDomain } => {
  const taken = new Set<number>();
  const fallthrough = new Set<number>();
  for (const value of state.a) {
    let predicate: boolean | null = null;
    if (state.flagSource?.kind === "a_zero") {
      if (condition === "z") predicate = value === 0;
      if (condition === "nz") predicate = value !== 0;
    } else if (state.flagSource?.kind === "a_compare") {
      if (condition === "z") predicate = value === state.flagSource.value;
      if (condition === "nz") predicate = value !== state.flagSource.value;
      if (condition === "c") predicate = value < state.flagSource.value;
      if (condition === "nc") predicate = value >= state.flagSource.value;
    }
    if (predicate !== false) taken.add(value);
    if (predicate !== true) fallthrough.add(value);
  }
  return { taken, fallthrough };
};

const evaluateByteOperand = (
  operand: string,
  constants: ReadonlyMap<string, number>,
): ByteDomain => {
  if (/^\[.*\]$/.test(operand)) return allByteValues();
  try {
    return singletonByte(evaluateAsmInteger(operand, constants));
  } catch {
    return allByteValues();
  }
};

const inferReturnAValues = (
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  entrypoint: string,
): ByteDomain => {
  if (!blocks.has(entrypoint)) {
    throw new Error(
      `Cannot infer return A for missing source entry ${entrypoint}`,
    );
  }
  const pending: AbstractByteState[] = [
    {
      block: entrypoint,
      instruction: 0,
      a: allByteValues(),
      flagSource: null,
    },
  ];
  const visited = new Set<string>();
  const returns = new Set<number>();
  const enqueue = (state: AbstractByteState): void => {
    if (state.a.size > 0 && !visited.has(domainKey(state))) pending.push(state);
  };
  while (pending.length > 0) {
    const state = pending.shift()!;
    const key = domainKey(state);
    if (visited.has(key)) continue;
    visited.add(key);
    const block = blocks.get(state.block)!;
    if (state.instruction >= block.instructions.length) {
      if (block.nextBlock) {
        enqueue({ ...state, block: block.nextBlock, instruction: 0 });
      }
      continue;
    }
    const instruction = block.instructions[state.instruction];
    const next = { ...state, instruction: state.instruction + 1 };
    if (
      ["ld", "ldh"].includes(instruction.opcode) &&
      instruction.args[0] === "a" &&
      instruction.args[1]
    ) {
      enqueue({
        ...next,
        a: evaluateByteOperand(instruction.args[1], constants),
      });
      continue;
    }
    if (
      instruction.opcode === "xor" &&
      instruction.args.length === 1 &&
      instruction.args[0] === "a"
    ) {
      enqueue({ ...next, a: singletonByte(0), flagSource: { kind: "a_zero" } });
      continue;
    }
    if (
      instruction.opcode === "and" &&
      instruction.args.length === 1 &&
      instruction.args[0] === "a"
    ) {
      enqueue({ ...next, flagSource: { kind: "a_zero" } });
      continue;
    }
    if (instruction.opcode === "cp" && instruction.args.length === 1) {
      try {
        enqueue({
          ...next,
          flagSource: {
            kind: "a_compare",
            value:
              ((evaluateAsmInteger(instruction.args[0], constants) % 256) +
                256) %
              256,
          },
        });
      } catch {
        enqueue({ ...next, flagSource: null });
      }
      continue;
    }
    if (instruction.opcode === "bit") {
      enqueue({ ...next, flagSource: null });
      continue;
    }
    if (["jr", "jp"].includes(instruction.opcode)) {
      const target = instructionTarget(block, instruction, blocks);
      if (!target || !blocks.has(target)) {
        throw new Error(
          `Return-domain interpreter cannot resolve ${instruction.opcode} target ${String(target)} at ${block.file}:${instruction.source_span.start_line}`,
        );
      }
      if (instruction.args.length === 1) {
        enqueue({ ...state, block: target, instruction: 0 });
      } else {
        const domains = conditionDomains(state, instruction.args[0]);
        enqueue({
          ...state,
          block: target,
          instruction: 0,
          a: domains.taken,
        });
        enqueue({ ...next, a: domains.fallthrough });
      }
      continue;
    }
    if (instruction.opcode === "ret") {
      if (instruction.args.length === 0) {
        for (const value of state.a) returns.add(value);
      } else {
        const domains = conditionDomains(state, instruction.args[0]);
        for (const value of domains.taken) returns.add(value);
        enqueue({ ...next, a: domains.fallthrough });
      }
      continue;
    }
    if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
      const target = instructionTarget(block, instruction, blocks);
      if (target === "CloseSRAM") {
        enqueue(next);
      } else {
        enqueue({ ...next, a: allByteValues(), flagSource: null });
      }
      continue;
    }
    enqueue(next);
  }
  return returns;
};

const releaseInstructions = (
  instructions: RuntimePresentationAsmInstruction[],
): RuntimePresentationAsmInstruction[] => {
  const output: RuntimePresentationAsmInstruction[] = [];
  const active = [true];
  for (const instruction of instructions) {
    if (instruction.opcode === "if") {
      const condition = instruction.args.join(",");
      if (condition !== "DEF(_DEBUG)") {
        throw new Error(
          `Unsupported runtime presentation source condition ${condition} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      active.push(active.at(-1)! && false);
      continue;
    }
    if (instruction.opcode === "else") {
      if (active.length === 1) {
        throw new Error("Runtime presentation source has unmatched else");
      }
      const parent = active.at(-2)!;
      active[active.length - 1] = parent && !active.at(-1)!;
      continue;
    }
    if (instruction.opcode === "endc") {
      if (active.length === 1) {
        throw new Error("Runtime presentation source has unmatched endc");
      }
      active.pop();
      continue;
    }
    if (active.at(-1)) output.push(instruction);
  }
  if (active.length !== 1) {
    throw new Error("Runtime presentation source has unterminated conditional");
  }
  return output;
};

const parseCountedByteRecords = (
  block: ParsedAsmBlock,
  constants: ReadonlyMap<string, number>,
): number[][] => {
  const values = releaseInstructions(block.instructions)
    .filter((instruction) => instruction.opcode === "db")
    .flatMap((instruction) =>
      instruction.args.map((argument) =>
        evaluateAsmInteger(argument, constants),
      ),
    );
  const records: number[][] = [];
  let cursor = 0;
  while (cursor < values.length) {
    const count = values[cursor++];
    if (!Number.isInteger(count) || count < 0) {
      throw new Error(
        `Runtime presentation data ${block.id} has invalid record length ${count}`,
      );
    }
    const items = values.slice(cursor, cursor + count);
    cursor += count;
    if (items.length !== count || values[cursor] !== -1) {
      throw new Error(
        `Runtime presentation data ${block.id} has a truncated or unterminated counted record`,
      );
    }
    cursor += 1;
    records.push(items.map((value) => ((value % 256) + 256) % 256));
  }
  return records;
};

const callSubtreeContainsExternalCall = (
  target: string,
  externalTarget: string,
  blocks: Map<string, ParsedAsmBlock>,
  visited: Set<string>,
): boolean => {
  if (visited.has(target)) return false;
  const block = blocks.get(target);
  if (!block) return false;
  visited.add(target);
  for (const instruction of block.instructions) {
    if (!["call", "callfar", "farcall"].includes(instruction.opcode)) continue;
    const child = instructionTarget(block, instruction, blocks);
    if (child === externalTarget) return true;
    if (
      child &&
      callSubtreeContainsExternalCall(
        child,
        externalTarget,
        blocks,
        new Set(visited),
      )
    ) {
      return true;
    }
  }
  return (
    !!block.nextBlock &&
    !isUnconditionalTerminal(block.instructions.at(-1)) &&
    callSubtreeContainsExternalCall(
      block.nextBlock,
      externalTarget,
      blocks,
      new Set(visited),
    )
  );
};

const inferTypedMenuSelectionDomain = (
  dispatchBlock: ParsedAsmBlock,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): ByteDomain | null => {
  const family = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === dispatchBlock.file &&
        candidate.globalLabel === dispatchBlock.globalLabel,
    )
    .sort((left, right) => left.startLine - right.startLine);
  const setup = family.flatMap((block) =>
    block.instructions.map((instruction) => ({ block, instruction })),
  );
  const selectionIndex = setup.findIndex(
    ({ instruction }) =>
      instruction.opcode === "ld" &&
      instruction.args[0] === "a" &&
      instruction.args[1] === "[wMenuSelection]",
  );
  if (selectionIndex < 0) return null;
  const producerIndex = setup.findIndex(
    ({ instruction }) =>
      instruction.opcode === "ld" &&
      instruction.args[0] === "[wWhichIndexSet]" &&
      instruction.args[1] === "a",
  );
  if (producerIndex <= 0) return null;
  const producerCall = [...setup.slice(0, producerIndex)]
    .reverse()
    .find(({ instruction }) =>
      ["call", "callfar", "farcall"].includes(instruction.opcode),
    );
  if (!producerCall) return null;
  const producerTarget = instructionTarget(
    producerCall.block,
    producerCall.instruction,
    blocks,
  );
  if (!producerTarget || !blocks.has(producerTarget)) return null;
  const menuKinds = inferReturnAValues(blocks, constants, producerTarget);

  const loadHeaderIndex = setup.findIndex(
    ({ instruction }, index) =>
      index > producerIndex &&
      index < selectionIndex &&
      ["call", "callfar", "farcall"].includes(instruction.opcode) &&
      instruction.args.at(-1) === "LoadMenuHeader",
  );
  if (loadHeaderIndex <= 0) return null;
  const headerPointer = [...setup.slice(0, loadHeaderIndex)]
    .reverse()
    .find(
      ({ instruction }) =>
        instruction.opcode === "ld" &&
        instruction.args[0] === "hl" &&
        !!instruction.args[1],
    );
  if (!headerPointer) return null;
  const headerTarget = resolveControlTarget(
    headerPointer.block,
    headerPointer.instruction.args[1],
    blocks,
    headerPointer.instruction.source_span.start_line,
  );
  const headerBlock = blocks.get(headerTarget);
  if (!headerBlock) return null;
  const menuDataTargets = headerBlock.instructions
    .filter((instruction) => instruction.opcode === "dw")
    .flatMap((instruction) =>
      instruction.args.map((argument) =>
        resolveControlTarget(
          headerBlock,
          argument,
          blocks,
          instruction.source_span.start_line,
        ),
      ),
    )
    .map((target) => blocks.get(target))
    .filter((target): target is ParsedAsmBlock => !!target);
  const countedRecordBlocks = menuDataTargets.flatMap((menuDataBlock) =>
    menuDataBlock.instructions
      .filter((instruction) => instruction.opcode === "dw")
      .flatMap((instruction) =>
        instruction.args.map((argument) =>
          resolveControlTarget(
            menuDataBlock,
            argument,
            blocks,
            instruction.source_span.start_line,
          ),
        ),
      )
      .map((target) => blocks.get(target))
      .filter((target): target is ParsedAsmBlock => !!target)
      .filter((candidate) => {
        try {
          return parseCountedByteRecords(candidate, constants).length > 0;
        } catch {
          return false;
        }
      }),
  );
  const uniqueRecordBlocks = [
    ...new Map(countedRecordBlocks.map((block) => [block.id, block])).values(),
  ];
  if (uniqueRecordBlocks.length !== 1) return null;
  const recordBlock = uniqueRecordBlocks[0];

  const hasTypedSelectionProducer = setup
    .slice(loadHeaderIndex + 1, selectionIndex)
    .some(({ block, instruction }) => {
      if (!["call", "callfar", "farcall"].includes(instruction.opcode)) {
        return false;
      }
      const target = instructionTarget(block, instruction, blocks);
      return (
        !!target &&
        callSubtreeContainsExternalCall(
          target,
          "GetScrollingMenuJoypad",
          blocks,
          new Set(),
        )
      );
    });
  if (!hasTypedSelectionProducer) return null;

  const records = parseCountedByteRecords(recordBlock, constants);
  const selections = new Set<number>();
  for (const menuKind of menuKinds) {
    const record = records[menuKind];
    if (!record) {
      throw new Error(
        `Runtime presentation menu kind ${menuKind} has no ${recordBlock.id} record`,
      );
    }
    for (const item of record) selections.add(item);
  }
  return selections;
};

const stateMachineActions = (
  handler: string,
  stateSymbol: string,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): Array<{ kind: "keep" | "increment" | "set_bit"; bit?: number }> => {
  const entry = blocks.get(handler);
  if (!entry) return [];
  const family = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === entry.file &&
        candidate.globalLabel === entry.globalLabel,
    )
    .sort((left, right) => left.startLine - right.startLine);
  const actions: Array<{
    kind: "keep" | "increment" | "set_bit";
    bit?: number;
  }> = [{ kind: "keep" }];
  for (const block of family) {
    let hlSymbol: string | null = null;
    for (const instruction of block.instructions) {
      if (
        instruction.opcode === "ld" &&
        instruction.args[0] === "hl" &&
        instruction.args[1]
      ) {
        hlSymbol = instruction.args[1];
        continue;
      }
      if (
        instruction.opcode === "inc" &&
        instruction.args[0] === "[hl]" &&
        hlSymbol === stateSymbol
      ) {
        actions.push({ kind: "increment" });
      }
      if (
        instruction.opcode === "set" &&
        instruction.args[1] === "[hl]" &&
        hlSymbol === stateSymbol
      ) {
        try {
          actions.push({
            kind: "set_bit",
            bit: evaluateAsmInteger(instruction.args[0], constants),
          });
        } catch {
          return [];
        }
      }
      if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
        const target = instructionTarget(block, instruction, blocks);
        const helper = target
          ? certifyOuterIncrementHelper(target, blocks)
          : null;
        if (helper?.symbol === stateSymbol) {
          actions.push({ kind: "increment" });
        }
      }
    }
  }
  return actions;
};

const instructionsInitializeStateToZero = (
  instructions: RuntimePresentationAsmInstruction[],
  stateSymbol: string,
): boolean => {
  let aIsZero = false;
  let hlSymbol: string | null = null;
  for (const instruction of instructions) {
    if (instruction.opcode === "xor" && instruction.args[0] === "a") {
      aIsZero = true;
    } else if (instruction.opcode === "ld" && instruction.args[0] === "hl") {
      hlSymbol = instruction.args[1];
    } else if (
      instruction.opcode === "ld" &&
      ["[hl]", "[hli]"].includes(instruction.args[0]) &&
      instruction.args[1] === "a" &&
      aIsZero &&
      hlSymbol === stateSymbol
    ) {
      return true;
    } else if (
      instruction.opcode === "ld" &&
      instruction.args[0] === `[${stateSymbol}]` &&
      instruction.args[1] === "a" &&
      aIsZero
    ) {
      return true;
    } else if (instruction.opcode === "ld" && instruction.args[0] === "a") {
      aIsZero = false;
    }
  }
  return false;
};

const callSubtreeInitializesStateToZero = (
  target: string,
  stateSymbol: string,
  blocks: Map<string, ParsedAsmBlock>,
  visited: Set<string>,
): boolean => {
  if (visited.has(target)) return false;
  const block = blocks.get(target);
  if (!block) return false;
  visited.add(target);
  if (instructionsInitializeStateToZero(block.instructions, stateSymbol)) {
    return true;
  }
  for (const instruction of block.instructions) {
    if (!["call", "callfar", "farcall"].includes(instruction.opcode)) continue;
    const child = instructionTarget(block, instruction, blocks);
    if (
      child &&
      callSubtreeInitializesStateToZero(
        child,
        stateSymbol,
        blocks,
        new Set(visited),
      )
    ) {
      return true;
    }
  }
  if (
    block.nextBlock &&
    !isUnconditionalTerminal(block.instructions.at(-1)) &&
    callSubtreeInitializesStateToZero(
      block.nextBlock,
      stateSymbol,
      blocks,
      new Set(visited),
    )
  ) {
    return true;
  }
  return false;
};

const stateIsInitializedBeforeDispatcherLoop = (
  callSite: { block: ParsedAsmBlock; index: number },
  stateSymbol: string,
  blocks: Map<string, ParsedAsmBlock>,
): boolean => {
  const localPrefix = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === callSite.block.file &&
        candidate.globalLabel === callSite.block.globalLabel &&
        candidate.startLine <= callSite.block.startLine,
    )
    .sort((left, right) => left.startLine - right.startLine)
    .map((candidate) => ({
      block: candidate,
      instructions:
        candidate === callSite.block
          ? candidate.instructions.slice(0, callSite.index)
          : candidate.instructions,
    }));
  const localInstructions = localPrefix.flatMap(
    ({ instructions }) => instructions,
  );
  if (instructionsInitializeStateToZero(localInstructions, stateSymbol)) {
    return true;
  }
  if (
    localPrefix.some(({ block, instructions }) =>
      instructions.some((instruction) => {
        if (!["call", "callfar", "farcall"].includes(instruction.opcode)) {
          return false;
        }
        const target = instructionTarget(block, instruction, blocks);
        return (
          !!target &&
          callSubtreeInitializesStateToZero(
            target,
            stateSymbol,
            blocks,
            new Set(),
          )
        );
      }),
    )
  ) {
    return true;
  }

  const controllerCalls = [...blocks.values()].flatMap((candidate) =>
    candidate.instructions.flatMap((instruction, index) => {
      if (!["call", "callfar", "farcall"].includes(instruction.opcode))
        return [];
      return instructionTarget(candidate, instruction, blocks) ===
        callSite.block.globalLabel
        ? [{ block: candidate, index }]
        : [];
    }),
  );
  return controllerCalls.some((controllerCall) => {
    const preceding = [...blocks.values()]
      .filter(
        (candidate) =>
          candidate.file === controllerCall.block.file &&
          candidate.globalLabel === controllerCall.block.globalLabel &&
          candidate.startLine <= controllerCall.block.startLine,
      )
      .sort((left, right) => left.startLine - right.startLine)
      .map((candidate) => ({
        block: candidate,
        instructions:
          candidate === controllerCall.block
            ? candidate.instructions.slice(0, controllerCall.index)
            : candidate.instructions,
      }));
    if (
      instructionsInitializeStateToZero(
        preceding.flatMap(({ instructions }) => instructions),
        stateSymbol,
      )
    ) {
      return true;
    }
    return preceding.some(({ block, instructions }) =>
      instructions.some((instruction) => {
        if (!["call", "callfar", "farcall"].includes(instruction.opcode)) {
          return false;
        }
        const target = instructionTarget(block, instruction, blocks);
        return (
          !!target &&
          callSubtreeInitializesStateToZero(
            target,
            stateSymbol,
            blocks,
            new Set(),
          )
        );
      }),
    );
  });
};

const inferStateMachineDomain = (
  dispatcher: ParsedAsmBlock,
  entries: string[],
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  additionalActions: ReadonlyMap<
    number,
    Array<{ kind: "keep" | "increment" | "set_bit"; bit?: number }>
  > = new Map(),
): ByteDomain | null => {
  const callSites = [...blocks.values()].flatMap((candidate) =>
    candidate.instructions.flatMap((instruction, index) => {
      const target = instructionTarget(candidate, instruction, blocks);
      return target === dispatcher.globalLabel
        ? [{ block: candidate, instruction, index }]
        : [];
    }),
  );
  if (callSites.length !== 1) return null;
  const callSite = callSites[0];
  const beforeCall = callSite.block.instructions.slice(0, callSite.index);
  const stateLoadIndex = [...beforeCall]
    .map((instruction, index) => ({ instruction, index }))
    .reverse()
    .find(
      ({ instruction }) =>
        instruction.opcode === "ld" &&
        instruction.args[0] === "a" &&
        /^\[[A-Za-z_][A-Za-z0-9_]*\]$/.test(instruction.args[1] ?? ""),
    );
  if (!stateLoadIndex) return null;
  const stateSymbol = stateLoadIndex.instruction.args[1].slice(1, -1);
  const guard = beforeCall
    .slice(stateLoadIndex.index + 1)
    .find(
      (instruction) =>
        instruction.opcode === "bit" && instruction.args[1] === "a",
    );
  const guardBranch = beforeCall
    .slice(stateLoadIndex.index + 1)
    .find(
      (instruction) =>
        ["jr", "jp"].includes(instruction.opcode) &&
        instruction.args[0] === "nz",
    );
  if (!guard || !guardBranch) return null;
  let exitBit: number;
  try {
    exitBit = evaluateAsmInteger(guard.args[0], constants);
  } catch {
    return null;
  }
  if (!stateIsInitializedBeforeDispatcherLoop(callSite, stateSymbol, blocks)) {
    return null;
  }

  const reached = new Set<number>([0]);
  const pending = [0];
  let reachedExit = false;
  while (pending.length > 0) {
    const value = pending.shift()!;
    if ((value & (1 << exitBit)) !== 0) {
      reachedExit = true;
      continue;
    }
    if (value >= entries.length) return null;
    const actions = [
      ...stateMachineActions(entries[value], stateSymbol, blocks, constants),
      ...(additionalActions.get(value) ?? []),
    ];
    if (actions.length === 0) return null;
    for (const action of actions) {
      const next =
        action.kind === "keep"
          ? value
          : action.kind === "increment"
            ? (value + 1) & 0xff
            : value | (1 << action.bit!);
      if (!reached.has(next)) {
        reached.add(next);
        pending.push(next);
      }
    }
  }
  if (!reachedExit) return null;
  return new Set(
    [...reached].filter((value) => (value & (1 << exitBit)) === 0),
  );
};

const parseAsmConstants = (sources: LoadedSource[]): Map<string, number> => {
  const constants = new Map<string, number>();
  constants.set("TRUE", 1);
  constants.set("FALSE", 0);
  for (const source of sources) {
    let constValue = 0;
    let rsValue = 0;
    for (const rawLine of source.lines) {
      const line = normalizeAsmLine(rawLine);
      if (line === "rsreset") {
        rsValue = 0;
        constants.set("_RS", rsValue);
        continue;
      }
      let match = line.match(/^const_def(?:\s+(.+))?$/);
      if (match) {
        constValue = match[1]
          ? evaluateAsmInteger(splitInstructionArgs(match[1])[0], constants)
          : 0;
        continue;
      }
      match = line.match(/^const\s+([A-Za-z_][A-Za-z0-9_]*)/);
      if (match) {
        constants.set(match[1], constValue);
        constValue += 1;
        continue;
      }
      match = line.match(/^const_skip(?:\s+(.+))?$/);
      if (match) {
        constValue += match[1] ? evaluateAsmInteger(match[1], constants) : 1;
        continue;
      }
      match = line.match(/^shift_const\s+([A-Za-z_][A-Za-z0-9_]*)/);
      if (match) {
        constants.set(match[1], 1 << constValue);
        constants.set(`${match[1]}_F`, constValue);
        constValue += 1;
        continue;
      }
      match = line.match(/^DEF\s+([A-Za-z_][A-Za-z0-9_]*)\s+rb(?:\s+(.+))?$/i);
      if (match) {
        constants.set(match[1], rsValue);
        rsValue += match[2] ? evaluateAsmInteger(match[2], constants) : 1;
        constants.set("_RS", rsValue);
        continue;
      }
      match = line.match(/^DEF\s+([A-Za-z_][A-Za-z0-9_]*)\s+EQU\s+(.+)$/i);
      if (match) {
        const expression = match[2].replace(
          /\bconst_value\b/g,
          String(constValue),
        );
        try {
          constants.set(match[1], evaluateAsmInteger(expression, constants));
        } catch {
          // Most source constants are unrelated to this program and can remain
          // symbolic. A table guard that needs one is rejected below.
        }
      }
    }
  }
  return constants;
};

const evaluateAsmInteger = (
  expression: string,
  constants: ReadonlyMap<string, number>,
): number => {
  let normalized = expression
    .replace(/DEF\(_DEBUG\)/g, "0")
    .replace(/\$([0-9a-f_]+)/gi, (_, digits: string) =>
      String(Number.parseInt(digits.replace(/_/g, ""), 16)),
    )
    .replace(/%([01_]+)/g, (_, digits: string) =>
      String(Number.parseInt(digits.replace(/_/g, ""), 2)),
    )
    .replace(/\b[A-Za-z_][A-Za-z0-9_]*\b/g, (name) => {
      const value = constants.get(name);
      if (value === undefined) {
        throw new Error(`Unresolved ASM integer ${name}`);
      }
      return String(value);
    });
  if (!/^[\d\s()+*/<>|&-]+$/.test(normalized)) {
    throw new Error(`Unsupported ASM integer expression ${expression}`);
  }
  // The whitelist above limits this expression to integer arithmetic only.
  const value = Function(`"use strict"; return (${normalized});`)() as unknown;
  if (!Number.isSafeInteger(value)) {
    throw new Error(
      `ASM integer expression is not a safe integer: ${expression}`,
    );
  }
  return value as number;
};

type SpriteDomainInferenceContext = {
  operations: RuntimePresentationSpriteOperation[];
  programs: RuntimePresentationSpriteProgram[];
  diagnostics: RuntimePresentationControlFlow["sprite_diagnostics"];
  loadSource: (relativePath: string) => LoadedSource;
};

type SpriteObjectRecord = {
  object: RuntimePresentationNamedByte & {
    table_source_span: RuntimePresentationSourceSpan;
  };
  frameset: RuntimePresentationNamedByte;
  callback: RuntimePresentationNamedByte;
  dictionary: RuntimePresentationNamedByte;
};

type InitializedSpriteInstance = {
  instance: string;
  structSlot: number;
  objectSymbol: string;
  objectValue: number;
  sourceSpan: RuntimePresentationSourceSpan;
  allocationSourceSpan: RuntimePresentationSourceSpan;
  allocationDispatchTick: number;
  framesetOverrideSymbol: string | null;
  memory: RuntimePresentationSpriteProgram["initial_memory"];
};

type SpriteCallbackDataResource =
  RuntimePresentationSpriteProgram["callback_data_resources"][number];

const parsedBlockSourceSpan = (
  block: ParsedAsmBlock,
): RuntimePresentationSourceSpan => ({
  file: block.file,
  start_line: block.startLine,
  end_line: block.instructions.at(-1)?.source_span.end_line ?? block.startLine,
});

const parseRgb555IncludedResource = (
  symbol: string,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): SpriteCallbackDataResource => {
  const block = blocks.get(symbol);
  if (!block) throw new Error(`RGB555 included resource ${symbol} is missing`);
  if (
    block.instructions.length !== 1 ||
    block.instructions[0].opcode.toUpperCase() !== "INCLUDE" ||
    block.instructions[0].args.length !== 1
  ) {
    throw new Error(
      `RGB555 included resource ${symbol} is not one exact source include`,
    );
  }
  const include = block.instructions[0];
  const quotedPath = include.args[0];
  const pathMatch = quotedPath.match(/^"([^"]+)"$/);
  if (!pathMatch) {
    throw new Error(
      `RGB555 included resource ${symbol} has malformed include ${quotedPath}`,
    );
  }
  const path = pathMatch[1];
  const source = loadSource(path);
  const colors: SpriteCallbackDataResource["colors"] = [];
  for (let lineIndex = 0; lineIndex < source.lines.length; lineIndex += 1) {
    const line = normalizeAsmLine(source.lines[lineIndex]);
    if (!line) continue;
    const match = line.match(/^RGB\s+(.+)$/i);
    if (!match) {
      throw new Error(
        `unsupported RGB555 palette data at ${source.file}:${lineIndex + 1}`,
      );
    }
    const components = splitInstructionArgs(match[1]);
    if (components.length !== 3) {
      throw new Error(
        `malformed RGB555 palette data at ${source.file}:${lineIndex + 1}`,
      );
    }
    const values = components.map((component) =>
      /^\d+$/.test(component)
        ? Number.parseInt(component, 10)
        : evaluateAsmInteger(component, constants),
    );
    if (values.some((value) => value < 0 || value > 31)) {
      throw new Error(
        `RGB555 palette data is out of range at ${source.file}:${lineIndex + 1}`,
      );
    }
    colors.push({
      red: values[0],
      green: values[1],
      blue: values[2],
      source_span: {
        file: source.file,
        start_line: lineIndex + 1,
        end_line: lineIndex + 1,
      },
    });
  }
  if (colors.length === 0) {
    throw new Error(`RGB555 included resource ${symbol} is empty`);
  }
  const bytes = colors.flatMap(({ red, green, blue }) => {
    const packed = red | (green << 5) | (blue << 10);
    return [packed & 0xff, packed >>> 8];
  });
  return {
    symbol,
    kind: "rgb555_palette",
    path,
    include_source_span: include.source_span,
    data_source_span: {
      file: source.file,
      start_line: colors[0].source_span.start_line,
      end_line: colors.at(-1)!.source_span.end_line,
    },
    bytes,
    colors,
  };
};

const instructionNamedByte = (
  symbol: string,
  constants: ReadonlyMap<string, number>,
): RuntimePresentationNamedByte => ({
  symbol,
  value: ((evaluateAsmInteger(symbol, constants) % 256) + 256) % 256,
});

const parseSpriteObjectRecord = (
  objectSymbol: string,
  objectValue: number,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): SpriteObjectRecord => {
  const table = blocks.get("SpriteAnimObjects");
  if (!table) {
    throw new Error("sprite object mapping SpriteAnimObjects is missing");
  }
  const rows = table.instructions.filter(
    (instruction) => instruction.opcode === "db",
  );
  const row = rows[objectValue];
  if (!row) {
    throw new Error(
      `sprite object ${objectSymbol}=${objectValue} has no object table row`,
    );
  }
  if (row.args.length !== 3) {
    throw new Error(
      `sprite object ${objectSymbol} row at ${row.source_span.file}:${row.source_span.start_line} must have exactly three fields`,
    );
  }
  let frameset: RuntimePresentationNamedByte;
  let callback: RuntimePresentationNamedByte;
  let dictionary: RuntimePresentationNamedByte;
  try {
    frameset = instructionNamedByte(row.args[0], constants);
  } catch (error) {
    throw new Error(
      `sprite frameset mapping ${row.args[0]} is unresolved: ${String(error)}`,
    );
  }
  try {
    callback = instructionNamedByte(row.args[1], constants);
  } catch (error) {
    throw new Error(
      `sprite function mapping ${row.args[1]} is unresolved: ${String(error)}`,
    );
  }
  try {
    dictionary = instructionNamedByte(row.args[2], constants);
  } catch (error) {
    throw new Error(
      `sprite dictionary mapping ${row.args[2]} is unresolved: ${String(error)}`,
    );
  }
  return {
    object: {
      symbol: objectSymbol,
      value: objectValue,
      table_source_span: row.source_span,
    },
    frameset,
    callback,
    dictionary,
  };
};

const certifyOamWaitMacro = (
  loadSource: (relativePath: string) => LoadedSource,
): RuntimePresentationSourceSpan => {
  const source = loadSource("macros/scripts/oam_anims.asm");
  const normalized = source.lines.map(normalizeAsmLine);
  const expected = [
    "const oamwait_command",
    "MACRO oamwait",
    "db oamwait_command",
    "db \\1",
    "ENDM",
  ];
  const starts = normalized.flatMap((_, start) =>
    expected.every((value, offset) => normalized[start + offset] === value)
      ? [start]
      : [],
  );
  if (starts.length !== 1) {
    throw new Error(
      "typed oamwait macro must emit its command byte and one exact duration byte",
    );
  }
  return {
    file: source.file,
    start_line: starts[0] + 1,
    end_line: starts[0] + expected.length,
  };
};

const parseSpriteFrameset = (
  frameset: RuntimePresentationNamedByte,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): RuntimePresentationSpriteProgram["frameset"] => {
  const table = blocks.get("SpriteAnimFrameData");
  if (!table) throw new Error("sprite frameset mapping table is missing");
  const tableRows = table.instructions.filter(
    (instruction) => instruction.opcode === "dw",
  );
  const tableRow = tableRows[frameset.value];
  if (!tableRow || tableRow.args.length !== 1) {
    throw new Error(
      `sprite frameset mapping ${frameset.symbol}=${frameset.value} is missing`,
    );
  }
  const target = resolveControlTarget(
    table,
    tableRow.args[0],
    blocks,
    tableRow.source_span.start_line,
  );
  const data = blocks.get(target);
  if (!data) {
    throw new Error(
      `sprite frameset mapping ${frameset.symbol} points to missing ${target}`,
    );
  }
  const frames: RuntimePresentationSpriteProgram["frameset"]["frames"] = [];
  const waits: RuntimePresentationSpriteProgram["frameset"]["waits"] = [];
  let terminal:
    RuntimePresentationSpriteProgram["frameset"]["terminal"] | null = null;
  for (const instruction of data.instructions) {
    if (instruction.opcode === "oamframe") {
      if (instruction.args.length < 2 || instruction.args.length > 4) {
        throw new Error(
          `sprite frameset ${target} has malformed oamframe at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      frames.push({
        oam_set: instructionNamedByte(instruction.args[0], constants),
        duration: evaluateAsmInteger(instruction.args[1], constants),
        flags:
          instruction.args.length > 2
            ? instruction.args.slice(2).join(" | ")
            : null,
        source_span: instruction.source_span,
      });
      continue;
    }
    if (instruction.opcode === "oamwait") {
      if (instruction.args.length !== 1) {
        throw new Error(
          `sprite frameset ${target} has malformed oamwait at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      const duration = evaluateAsmInteger(instruction.args[0], constants);
      if (duration < 0 || duration > 0xff) {
        throw new Error(
          `sprite frameset ${target} has out-of-range oamwait at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      waits.push({
        duration,
        source_span: instruction.source_span,
        implementation_source_span: certifyOamWaitMacro(loadSource),
      });
      continue;
    }
    if (
      instruction.opcode === "oamend" ||
      instruction.opcode === "oamrestart" ||
      instruction.opcode === "oamdelete"
    ) {
      terminal = {
        op:
          instruction.opcode === "oamend"
            ? "end"
            : instruction.opcode === "oamrestart"
              ? "restart"
              : "delete",
        source_span: instruction.source_span,
      };
      continue;
    }
    throw new Error(
      `unsupported sprite frameset opcode ${instruction.opcode} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
    );
  }
  if ((frames.length === 0 && waits.length === 0) || !terminal) {
    throw new Error(`sprite frameset ${target} is empty or unterminated`);
  }
  return {
    ...frameset,
    table_source_span: tableRow.source_span,
    data_source_span: parsedBlockSourceSpan(data),
    frames,
    waits,
    terminal,
  };
};

const parseSpriteOamResources = (
  frames: RuntimePresentationSpriteProgram["frameset"]["frames"],
  blocks: Map<string, ParsedAsmBlock>,
): RuntimePresentationSpriteProgram["oam_resources"] => {
  const table = blocks.get("SpriteAnimOAMData");
  if (!table) throw new Error("sprite OAM mapping table is missing");
  const rows = table.instructions.filter(
    (instruction) => instruction.opcode === "spriteanimoam",
  );
  const uniqueSets = [
    ...new Map(
      frames.map((frame) => [frame.oam_set.value, frame.oam_set]),
    ).values(),
  ];
  return uniqueSets.map((oamSet) => {
    const row = rows[oamSet.value];
    if (!row || row.args.length !== 2) {
      throw new Error(
        `sprite OAM mapping ${oamSet.symbol}=${oamSet.value} is missing`,
      );
    }
    const target = resolveControlTarget(
      table,
      row.args[1],
      blocks,
      row.source_span.start_line,
    );
    const data = blocks.get(target);
    if (!data) {
      throw new Error(
        `sprite OAM mapping ${oamSet.symbol} points to missing ${target}`,
      );
    }
    const sprites = data.instructions.filter(
      (instruction) => instruction.opcode === "dbsprite",
    );
    const countInstruction = data.instructions.find(
      (instruction) => instruction.opcode === "db",
    );
    if (!countInstruction || countInstruction.args.length !== 1) {
      throw new Error(`sprite OAM data ${target} has no exact count`);
    }
    const count = Number(countInstruction.args[0]);
    if (!Number.isInteger(count) || sprites.length !== count) {
      throw new Error(
        `sprite OAM data ${target} count ${countInstruction.args[0]} does not match ${sprites.length} records`,
      );
    }
    return {
      oam_set: oamSet,
      tile_offset: evaluateAsmInteger(row.args[0], new Map()),
      data_target: target,
      table_source_span: row.source_span,
      data_source_span: parsedBlockSourceSpan(data),
      sprites,
    };
  });
};

const defaultSpriteMemory =
  (): RuntimePresentationSpriteProgram["initial_memory"] => ({
    index: 1,
    frameset_id: 0,
    anim_seq_id: 0,
    tile_id: 0,
    xcoord: 0,
    ycoord: 0,
    xoffset: 0,
    yoffset: 0,
    duration: 0,
    duration_offset: 0,
    frame: 0xff,
    jumptable_index: 0,
    var1: 0,
    var2: 0,
    var3: 0,
    var4: 0,
  });

const spriteMemoryProperty = (
  symbol: string,
): keyof RuntimePresentationSpriteProgram["initial_memory"] | null => {
  const suffix = symbol.replace(/^SPRITEANIMSTRUCT_/, "");
  const properties: Record<
    string,
    keyof RuntimePresentationSpriteProgram["initial_memory"]
  > = {
    INDEX: "index",
    FRAMESET_ID: "frameset_id",
    ANIM_SEQ_ID: "anim_seq_id",
    TILE_ID: "tile_id",
    XCOORD: "xcoord",
    YCOORD: "ycoord",
    XOFFSET: "xoffset",
    YOFFSET: "yoffset",
    DURATION: "duration",
    DURATIONOFFSET: "duration_offset",
    FRAME: "frame",
    JUMPTABLE_INDEX: "jumptable_index",
    VAR1: "var1",
    VAR2: "var2",
    VAR3: "var3",
    VAR4: "var4",
  };
  return properties[suffix] ?? null;
};

const evaluateKnownByte = (
  value: string,
  constants: ReadonlyMap<string, number>,
): number => ((evaluateAsmInteger(value, constants) % 256) + 256) % 256;

const applyCallerSpriteOverrides = (
  block: ParsedAsmBlock,
  callIndex: number,
  memory: RuntimePresentationSpriteProgram["initial_memory"],
  constants: ReadonlyMap<string, number>,
): void => {
  let property:
    keyof RuntimePresentationSpriteProgram["initial_memory"] | null = null;
  for (const instruction of block.instructions.slice(callIndex + 1)) {
    if (["call", "callfar", "farcall", "ret"].includes(instruction.opcode)) {
      break;
    }
    if (
      (instruction.opcode === "ld" || instruction.opcode === "ldh") &&
      instruction.args[0] === "hl" &&
      instruction.args[1]
    ) {
      property = spriteMemoryProperty(instruction.args[1]);
      continue;
    }
    if (
      (instruction.opcode === "ld" || instruction.opcode === "ldh") &&
      instruction.args[0] === "[hl]" &&
      instruction.args[1] &&
      property
    ) {
      memory[property] = evaluateKnownByte(instruction.args[1], constants);
    }
  }
};

const certifyBulkSpriteDeinitializer = (
  blocks: Map<string, ParsedAsmBlock>,
): RuntimePresentationSourceSpan => {
  const entry = blocks.get("DeinitializeAllSprites");
  if (!entry) {
    throw new Error("typed DeinitializeAllSprites source is missing");
  }
  const family = handlerFamily("DeinitializeAllSprites", blocks);
  const instructions = family.flatMap((block) => block.instructions);
  const expected = [
    ["ld", "hl", "wSpriteAnimationStructs"],
    ["ld", "bc", "SPRITEANIMSTRUCT_LENGTH"],
    ["ld", "e", "NUM_SPRITE_ANIM_STRUCTS"],
    ["xor", "a"],
    ["ld", "[hl]", "a"],
    ["add", "hl", "bc"],
    ["dec", "e"],
    ["jr", "nz", ".loop"],
    ["ret"],
  ];
  if (
    instructions.length !== expected.length ||
    instructions.some(
      (instruction, index) =>
        [instruction.opcode, ...instruction.args].join("\0") !==
        expected[index].join("\0"),
    ) ||
    instructionTarget(entry, instructions[7], blocks) !==
      ".loop@DeinitializeAllSprites"
  ) {
    throw new Error(
      "typed DeinitializeAllSprites must clear every struct index",
    );
  }
  return {
    file: entry.file,
    start_line: entry.startLine,
    end_line: instructions.at(-1)!.source_span.end_line,
  };
};

const certifyClearSpriteAnimations = (
  blocks: Map<string, ParsedAsmBlock>,
): RuntimePresentationSourceSpan => {
  const entry = blocks.get("ClearSpriteAnims");
  if (!entry) throw new Error("typed ClearSpriteAnims source is missing");
  const instructions = handlerFamily("ClearSpriteAnims", blocks).flatMap(
    (block) => block.instructions,
  );
  const expected = [
    ["ld", "hl", "wSpriteAnimData"],
    ["ld", "bc", "wSpriteAnimDataEnd - wSpriteAnimData"],
    ["ld", "[hl]", "0"],
    ["inc", "hl"],
    ["dec", "bc"],
    ["ld", "a", "c"],
    ["or", "b"],
    ["jr", "nz", ".loop"],
    ["ret"],
  ];
  if (
    instructions.length !== expected.length ||
    instructions.some(
      (instruction, index) =>
        [instruction.opcode, ...instruction.args].join("\0") !==
        expected[index].join("\0"),
    ) ||
    instructionTarget(entry, instructions[7], blocks) !==
      ".loop@ClearSpriteAnims"
  ) {
    throw new Error(
      "typed ClearSpriteAnims must clear the complete animation state and count",
    );
  }
  return {
    file: entry.file,
    start_line: entry.startLine,
    end_line: instructions.at(-1)!.source_span.end_line,
  };
};

const validateSpriteRuntimeContracts = (
  blocks: Map<string, ParsedAsmBlock>,
  loadSource: (relativePath: string) => LoadedSource,
): void => {
  const initializer = blocks.get("_InitSpriteAnimStruct");
  if (!initializer)
    throw new Error("typed sprite initializer source is missing");
  const initializerInstructions = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === initializer.file &&
        candidate.globalLabel === initializer.globalLabel,
    )
    .sort((left, right) => left.startLine - right.startLine)
    .flatMap((candidate) => candidate.instructions);
  const initializerText = initializerInstructions
    .flatMap((instruction) => instruction.args)
    .join(" ");
  for (const required of [
    "SpriteAnimObjects",
    "SPRITEANIMSTRUCT_INDEX",
    "SPRITEANIMSTRUCT_XCOORD",
    "wSpriteAnimCount",
  ]) {
    if (!initializerText.includes(required)) {
      throw new Error(`typed sprite initializer omits ${required}`);
    }
  }
  const initializerSignatures = initializerInstructions.map((instruction) =>
    [instruction.opcode, ...instruction.args].join("\0"),
  );
  const hasInitializerSequence = (expected: string[][]): boolean =>
    initializerSignatures.some((_, start) =>
      expected.every(
        (parts, offset) =>
          initializerSignatures[start + offset] === parts.join("\0"),
      ),
    );
  if (
    !hasInitializerSequence([
      ["ld", "hl", "wSpriteAnimCount"],
      ["inc", "[hl]"],
      ["ld", "a", "[hl]"],
      ["and", "a"],
      ["jr", "nz", ".nonzero"],
      ["inc", "[hl]"],
    ]) ||
    !hasInitializerSequence([
      ["ld", "a", "[wSpriteAnimCount]"],
      ["ld", "[hli]", "a"],
    ]) ||
    !hasInitializerSequence([
      ["ld", "a", "[hl]"],
      ["and", "a"],
      ["jr", "z", ".found"],
      ["ld", "bc", "SPRITEANIMSTRUCT_LENGTH"],
      ["add", "hl", "bc"],
    ])
  ) {
    throw new Error(
      "typed sprite initializer must scan the first free slot and assign the exact nonzero animation count",
    );
  }
  const scheduler = blocks.get("DoNextFrameForAllSprites");
  if (!scheduler) throw new Error("typed sprite scheduler source is missing");
  const schedulerInstructions = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === scheduler.file &&
        candidate.globalLabel === scheduler.globalLabel,
    )
    .sort((left, right) => left.startLine - right.startLine)
    .flatMap((candidate) => candidate.instructions);
  const calls = schedulerInstructions
    .filter((instruction) => instruction.opcode === "call")
    .map((instruction) => instruction.args[0]);
  const callbackIndex = calls.indexOf("DoSpriteAnimFrame");
  const frameIndex = calls.indexOf("UpdateAnimFrame");
  if (callbackIndex < 0 || frameIndex < 0 || frameIndex !== callbackIndex + 1) {
    throw new Error(
      "typed sprite scheduler must execute callback before frame/OAM update",
    );
  }

  const reinitWrapper = blocks.get("ReinitSpriteAnimFrame");
  const reinitializer = blocks.get("_ReinitSpriteAnimFrame");
  if (!reinitWrapper || !reinitializer) {
    throw new Error("typed sprite frame reinitializer source is missing");
  }
  if (
    !reinitWrapper.instructions.some(
      (instruction) =>
        instruction.opcode === "call" &&
        instructionTarget(reinitWrapper, instruction, blocks) ===
          "_ReinitSpriteAnimFrame",
    )
  ) {
    throw new Error(
      "typed sprite frame reinitializer wrapper omits the engine call",
    );
  }
  const reinitInstructions = reinitializer.instructions;
  const expectedReinit = [
    ["ld", "hl", "SPRITEANIMSTRUCT_FRAMESET_ID"],
    ["add", "hl", "bc"],
    ["ld", "[hl]", "a"],
    ["ld", "hl", "SPRITEANIMSTRUCT_DURATION"],
    ["add", "hl", "bc"],
    ["ld", "[hl]", "0"],
    ["ld", "hl", "SPRITEANIMSTRUCT_FRAME"],
    ["add", "hl", "bc"],
    ["ld", "[hl]", "-1"],
    ["ret"],
  ];
  if (
    reinitInstructions.length !== expectedReinit.length ||
    reinitInstructions.some(
      (instruction, index) =>
        [instruction.opcode, ...instruction.args].join("\0") !==
        expectedReinit[index].join("\0"),
    )
  ) {
    throw new Error(
      "typed sprite frame reinitializer must set frameset, zero duration, and frame -1",
    );
  }

  const updateFrame = blocks.get("UpdateAnimFrame");
  const deleteFrame = blocks.get(".delete@UpdateAnimFrame");
  const deinitialize = blocks.get("DeinitializeSprite");
  if (!updateFrame || !deleteFrame || !deinitialize) {
    throw new Error("typed sprite delete lifetime source is missing");
  }
  const waitCompare = updateFrame.instructions.findIndex(
    (instruction) =>
      instruction.opcode === "cp" && instruction.args[0] === "oamwait_command",
  );
  const waitBranch = updateFrame.instructions[waitCompare + 1];
  if (
    waitCompare < 0 ||
    waitBranch?.opcode !== "jr" ||
    waitBranch.args[0] !== "z" ||
    instructionTarget(updateFrame, waitBranch, blocks) !==
      ".done@UpdateAnimFrame"
  ) {
    throw new Error(
      "typed oamwait must skip OAM generation through the exact UpdateAnimFrame done path",
    );
  }
  certifyOamWaitMacro(loadSource);
  const deleteCompare = updateFrame.instructions.findIndex(
    (instruction) =>
      instruction.opcode === "cp" &&
      instruction.args[0] === "oamdelete_command",
  );
  const deleteBranch = updateFrame.instructions[deleteCompare + 1];
  if (
    deleteCompare < 0 ||
    deleteBranch?.opcode !== "jr" ||
    deleteBranch.args[0] !== "z" ||
    instructionTarget(updateFrame, deleteBranch, blocks) !==
      ".delete@UpdateAnimFrame" ||
    !deleteFrame.instructions.some(
      (instruction) =>
        instruction.opcode === "call" &&
        instructionTarget(deleteFrame, instruction, blocks) ===
          "DeinitializeSprite",
    )
  ) {
    throw new Error(
      "typed oamdelete must branch to exact sprite deinitialization",
    );
  }
  const deinitializeSignatures = deinitialize.instructions.map((instruction) =>
    [instruction.opcode, ...instruction.args].join("\0"),
  );
  if (
    deinitializeSignatures.join("\n") !==
    [
      ["ld", "hl", "SPRITEANIMSTRUCT_INDEX"],
      ["add", "hl", "bc"],
      ["ld", "[hl]", "0"],
      ["ret"],
    ]
      .map((parts) => parts.join("\0"))
      .join("\n")
  ) {
    throw new Error(
      "typed sprite deinitialization must clear only the struct index",
    );
  }
  certifyBulkSpriteDeinitializer(blocks);
  certifyClearSpriteAnimations(blocks);
};

const collectInitializedSprites = (
  target: string,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  visited: Set<string>,
): { instances: InitializedSpriteInstance[]; cleared: boolean } => {
  if (visited.has(target)) return { instances: [], cleared: false };
  const block = blocks.get(target);
  if (!block) return { instances: [], cleared: false };
  visited.add(target);
  const instances: InitializedSpriteInstance[] = [];
  let cleared = false;
  let a: number | null = null;
  let dictionaryOffset: number | null = null;
  let dictionaryBytes: number[] | null = null;
  for (let index = 0; index < block.instructions.length; index += 1) {
    const instruction = block.instructions[index];
    if (instruction.opcode === "xor" && instruction.args[0] === "a") {
      a = 0;
      continue;
    }
    if (instruction.opcode === "ld" || instruction.opcode === "ldh") {
      const [destination, source] = instruction.args;
      if (destination === "a" && source) {
        try {
          a = source.startsWith("[")
            ? null
            : evaluateKnownByte(source, constants);
        } catch {
          a = null;
        }
        continue;
      }
      if (destination === "hl") {
        dictionaryOffset = source === "wSpriteAnimDict" ? 0 : null;
        continue;
      }
      if (
        (destination === "[hl]" || destination === "[hli]") &&
        dictionaryOffset !== null &&
        source
      ) {
        if (!dictionaryBytes) {
          throw new Error(
            `sprite dictionary write at ${instruction.source_span.file}:${instruction.source_span.start_line} has no exact ClearSpriteAnims reset`,
          );
        }
        const value =
          source === "a" && a !== null
            ? a
            : evaluateKnownByte(source, constants);
        if (dictionaryOffset >= dictionaryBytes.length) {
          throw new Error(
            `sprite dictionary write at ${instruction.source_span.file}:${instruction.source_span.start_line} exceeds its exact source byte span`,
          );
        }
        dictionaryBytes[dictionaryOffset] = value;
        if (destination === "[hli]") dictionaryOffset += 1;
        continue;
      }
    }
    if (instruction.opcode === "inc" && instruction.args[0] === "hl") {
      if (dictionaryOffset !== null) dictionaryOffset += 1;
      continue;
    }
    if (!["call", "callfar", "farcall"].includes(instruction.opcode)) continue;
    const child = instructionTarget(block, instruction, blocks);
    if (child === "ClearSpriteAnims") {
      cleared = true;
      const dictionaryEntries = constants.get("NUM_SPRITEANIMDICT_ENTRIES");
      if (dictionaryEntries === undefined || dictionaryEntries <= 0) {
        throw new Error(
          "sprite dictionary reset cannot resolve NUM_SPRITEANIMDICT_ENTRIES",
        );
      }
      dictionaryBytes = Array<number>(dictionaryEntries * 2).fill(0);
      a = null;
      dictionaryOffset = null;
      continue;
    }
    if (child === "InitSpriteAnimStruct") {
      const objectLoad = [...block.instructions.slice(0, index)]
        .reverse()
        .find(
          (candidate) =>
            candidate.opcode === "ld" &&
            candidate.args[0] === "a" &&
            !!candidate.args[1],
        );
      if (!objectLoad) {
        throw new Error(
          `typed sprite initializer at ${instruction.source_span.file}:${instruction.source_span.start_line} has no exact object id`,
        );
      }
      const objectSymbol = objectLoad.args[1];
      let objectValue: number;
      try {
        objectValue = evaluateKnownByte(objectSymbol, constants);
      } catch (error) {
        throw new Error(
          `sprite object id ${objectSymbol} is unresolved: ${String(error)}`,
        );
      }
      const memory = defaultSpriteMemory();
      const object = parseSpriteObjectRecord(
        objectSymbol,
        objectValue,
        blocks,
        constants,
      );
      if (!dictionaryBytes) {
        throw new Error(
          `sprite dictionary ${object.dictionary.symbol} at ${instruction.source_span.file}:${instruction.source_span.start_line} has no exact ClearSpriteAnims reset or initialized tile mapping`,
        );
      }
      let tileId: number | undefined;
      for (let offset = 0; offset < dictionaryBytes.length; offset += 2) {
        if (dictionaryBytes[offset] === object.dictionary.value) {
          tileId = dictionaryBytes[offset + 1];
          break;
        }
      }
      tileId ??= 0;
      memory.tile_id = tileId;
      const depixel = [...block.instructions.slice(0, index)]
        .reverse()
        .find((candidate) => candidate.opcode === "depixel");
      if (!depixel || depixel.args.length < 2 || depixel.args.length > 4) {
        throw new Error(
          `typed sprite initializer at ${instruction.source_span.file}:${instruction.source_span.start_line} has no exact depixel coordinates`,
        );
      }
      const tileWidth = constants.get("TILE_WIDTH");
      if (tileWidth === undefined) {
        throw new Error("typed sprite initializer cannot resolve TILE_WIDTH");
      }
      memory.ycoord =
        evaluateAsmInteger(depixel.args[0], constants) * tileWidth +
        (depixel.args[2] ? evaluateAsmInteger(depixel.args[2], constants) : 0);
      memory.xcoord =
        evaluateAsmInteger(depixel.args[1], constants) * tileWidth +
        (depixel.args[3] ? evaluateAsmInteger(depixel.args[3], constants) : 0);
      applyCallerSpriteOverrides(block, index, memory, constants);
      instances.push({
        instance: `sprite:${instruction.source_span.file}:${instruction.source_span.start_line}`,
        structSlot: 0,
        objectSymbol,
        objectValue,
        sourceSpan: instruction.source_span,
        allocationSourceSpan: instruction.source_span,
        allocationDispatchTick: 0,
        framesetOverrideSymbol: null,
        memory,
      });
      a = null;
      dictionaryOffset = null;
      continue;
    }
    if (child && blocks.has(child)) {
      const nested = collectInitializedSprites(
        child,
        blocks,
        constants,
        new Set(visited),
      );
      instances.push(...nested.instances);
      cleared ||= nested.cleared;
    }
    a = null;
    dictionaryOffset = null;
  }
  if (block.nextBlock && !isUnconditionalTerminal(block.instructions.at(-1))) {
    const nested = collectInitializedSprites(
      block.nextBlock,
      blocks,
      constants,
      new Set(visited),
    );
    instances.push(...nested.instances);
    cleared ||= nested.cleared;
  }
  return { instances, cleared };
};

type CallbackOuterEffect =
  RuntimePresentationSpriteProgram["outer_state_effects"][number];

type CallbackRegisterState = {
  a: number;
  d: number;
  e: number;
  zero: boolean;
  carry: boolean;
  hl:
    | {
        kind: "struct";
        property: keyof RuntimePresentationSpriteProgram["initial_memory"];
      }
    | { kind: "outer"; symbol: string }
    | { kind: "data"; symbol: string; offset: number }
    | null;
  stack: Array<{ a: number; zero: boolean; carry: boolean }>;
};

const callbackCondition = (
  condition: string,
  registers: CallbackRegisterState,
): boolean => {
  if (condition === "z") return registers.zero;
  if (condition === "nz") return !registers.zero;
  if (condition === "c") return registers.carry;
  if (condition === "nc") return !registers.carry;
  throw new Error(`unsupported sprite callback condition ${condition}`);
};

const readCallbackMemory = (
  registers: CallbackRegisterState,
  memory: RuntimePresentationSpriteProgram["initial_memory"],
  outerMemory: Map<string, number>,
  dataResources: ReadonlyMap<string, SpriteCallbackDataResource>,
): number => {
  if (!registers.hl) throw new Error("sprite callback reads unresolved [hl]");
  if (registers.hl.kind === "struct") {
    return memory[registers.hl.property];
  }
  if (registers.hl.kind === "outer") {
    return outerMemory.get(registers.hl.symbol) ?? 0;
  }
  const resource = dataResources.get(registers.hl.symbol);
  if (!resource) {
    throw new Error(
      `sprite callback reads unresolved data ${registers.hl.symbol}`,
    );
  }
  const value = resource.bytes[registers.hl.offset];
  if (value === undefined) {
    throw new Error(
      `sprite callback data ${registers.hl.symbol} offset ${registers.hl.offset} is outside its ${resource.bytes.length} bytes`,
    );
  }
  return value;
};

const writeCallbackMemory = (
  registers: CallbackRegisterState,
  memory: RuntimePresentationSpriteProgram["initial_memory"],
  outerMemory: Map<string, number>,
  value: number,
): void => {
  if (!registers.hl) throw new Error("sprite callback writes unresolved [hl]");
  const byte = value & 0xff;
  if (registers.hl.kind === "struct") {
    memory[registers.hl.property] = byte;
  } else if (registers.hl.kind === "outer") {
    outerMemory.set(registers.hl.symbol, byte);
  } else {
    throw new Error(
      `sprite callback writes read-only data ${registers.hl.symbol}`,
    );
  }
};

const certifyOuterIncrementHelper = (
  target: string,
  blocks: Map<string, ParsedAsmBlock>,
): { symbol: string; sourceSpan: RuntimePresentationSourceSpan } | null => {
  const block = blocks.get(target);
  if (!block) return null;
  const family = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === block.file &&
        candidate.globalLabel === block.globalLabel,
    )
    .sort((left, right) => left.startLine - right.startLine);
  const instructions = family.flatMap((candidate) => candidate.instructions);
  if (instructions.length !== 3) return null;
  const [load, increment, ret] = instructions;
  if (
    load.opcode !== "ld" ||
    load.args[0] !== "hl" ||
    !/^w[A-Za-z0-9_]+$/.test(load.args[1] ?? "") ||
    increment.opcode !== "inc" ||
    increment.args[0] !== "[hl]" ||
    ret.opcode !== "ret" ||
    ret.args.length !== 0
  ) {
    return null;
  }
  return { symbol: load.args[1], sourceSpan: parsedBlockSourceSpan(block) };
};

const callbackOperandByte = (
  operand: string,
  registers: CallbackRegisterState,
  constants: ReadonlyMap<string, number>,
): number => {
  if (operand === "a") return registers.a;
  if (operand === "d") return registers.d;
  if (operand === "e") return registers.e;
  if (/^BANK\(.+\)$/.test(operand)) return 0;
  return evaluateKnownByte(operand, constants);
};

const TYPED_SPRITE_CALLBACK_MEMORY_WRITES = new Set([
  "[rWBK]",
  "[wOBPals2 + 12]",
  "[wOBPals2 + 13]",
  "[hCGBPalUpdate]",
]);

const executeSpriteCallbackTick = (
  entry: string,
  fromCallbackIndex: number,
  memory: RuntimePresentationSpriteProgram["initial_memory"],
  outerMemory: Map<string, number>,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  dataResources: ReadonlyMap<string, SpriteCallbackDataResource>,
): CallbackOuterEffect[] => {
  const registers: CallbackRegisterState = {
    a: 0,
    d: 0,
    e: 0,
    zero: false,
    carry: false,
    hl: null,
    stack: [],
  };
  const effects: CallbackOuterEffect[] = [];
  let blockId = entry;
  let instructionIndex = 0;
  for (let steps = 0; steps < 500; steps += 1) {
    const block = blocks.get(blockId);
    if (!block) throw new Error(`sprite callback block ${blockId} is missing`);
    if (instructionIndex >= block.instructions.length) {
      if (!block.nextBlock) {
        throw new Error(`sprite callback ${blockId} falls out of source`);
      }
      blockId = block.nextBlock;
      instructionIndex = 0;
      continue;
    }
    const instruction = block.instructions[instructionIndex++];
    const fail = (): never => {
      throw new Error(
        `unsupported sprite callback opcode ${instruction.opcode} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    };
    if (instruction.opcode === "ld" || instruction.opcode === "ldh") {
      const [destination, source] = instruction.args;
      if (!destination || !source) fail();
      if (destination === "hl") {
        const property = spriteMemoryProperty(source);
        const dataTarget = resolveControlTarget(
          block,
          source,
          blocks,
          instruction.source_span.start_line,
        );
        registers.hl = property
          ? { kind: "struct", property }
          : /^w[A-Za-z0-9_]+$/.test(source)
            ? { kind: "outer", symbol: source }
            : dataResources.has(dataTarget)
              ? { kind: "data", symbol: dataTarget, offset: 0 }
              : null;
        if (!registers.hl) fail();
        continue;
      }
      if (destination === "a") {
        registers.a =
          source === "[hl]" || source === "[hli]"
            ? readCallbackMemory(registers, memory, outerMemory, dataResources)
            : /^\[.+\]$/.test(source)
              ? source === "[rWBK]"
                ? 0
                : fail()
              : callbackOperandByte(source, registers, constants);
        if (source === "[hli]" && registers.hl?.kind === "data") {
          registers.hl.offset += 1;
        }
        continue;
      }
      if (destination === "d" || destination === "e") {
        registers[destination] = callbackOperandByte(
          source,
          registers,
          constants,
        );
        continue;
      }
      if (destination === "de") {
        registers.d = 0;
        registers.e = 0;
        continue;
      }
      if (destination === "[hl]" || destination === "[hli]") {
        writeCallbackMemory(
          registers,
          memory,
          outerMemory,
          callbackOperandByte(source, registers, constants),
        );
        continue;
      }
      if (/^\[.+\]$/.test(destination)) {
        if (!TYPED_SPRITE_CALLBACK_MEMORY_WRITES.has(destination)) fail();
        callbackOperandByte(source, registers, constants);
        continue;
      }
      fail();
    }
    if (instruction.opcode === "add") {
      if (instruction.args[0] === "hl") {
        if (instruction.args[1] === "de" && registers.hl?.kind === "data") {
          registers.hl.offset += (registers.d << 8) | registers.e;
        }
        continue;
      }
      registers.a =
        (registers.a +
          callbackOperandByte(instruction.args[0], registers, constants)) &
        0xff;
      registers.zero = registers.a === 0;
      continue;
    }
    if (instruction.opcode === "inc" || instruction.opcode === "dec") {
      if (instruction.args[0] === "[hl]") {
        const delta = instruction.opcode === "inc" ? 1 : -1;
        const value =
          (readCallbackMemory(registers, memory, outerMemory, dataResources) +
            delta) &
          0xff;
        writeCallbackMemory(registers, memory, outerMemory, value);
        registers.zero = value === 0;
        continue;
      }
      if (instruction.args[0] === "a") {
        registers.a =
          (registers.a + (instruction.opcode === "inc" ? 1 : -1)) & 0xff;
        registers.zero = registers.a === 0;
        continue;
      }
      fail();
    }
    if (instruction.opcode === "and") {
      registers.a &= callbackOperandByte(
        instruction.args[0],
        registers,
        constants,
      );
      registers.zero = registers.a === 0;
      registers.carry = false;
      continue;
    }
    if (instruction.opcode === "xor") {
      registers.a =
        instruction.args[0] === "a"
          ? 0
          : registers.a ^
            callbackOperandByte(instruction.args[0], registers, constants);
      registers.zero = registers.a === 0;
      registers.carry = false;
      continue;
    }
    if (instruction.opcode === "sub") {
      const operand = callbackOperandByte(
        instruction.args[0],
        registers,
        constants,
      );
      registers.carry = registers.a < operand;
      registers.a = (registers.a - operand) & 0xff;
      registers.zero = registers.a === 0;
      continue;
    }
    if (instruction.opcode === "cp") {
      const operand = callbackOperandByte(
        instruction.args[0],
        registers,
        constants,
      );
      registers.zero = registers.a === operand;
      registers.carry = registers.a < operand;
      continue;
    }
    if (instruction.opcode === "srl" && instruction.args[0] === "a") {
      registers.carry = (registers.a & 1) !== 0;
      registers.a >>>= 1;
      registers.zero = registers.a === 0;
      continue;
    }
    if (instruction.opcode === "push" && instruction.args[0] === "af") {
      registers.stack.push({
        a: registers.a,
        zero: registers.zero,
        carry: registers.carry,
      });
      continue;
    }
    if (instruction.opcode === "pop" && instruction.args[0] === "af") {
      const saved = registers.stack.pop();
      if (!saved) {
        throw new Error(
          `sprite callback stack underflow at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      registers.a = saved.a;
      registers.zero = saved.zero;
      registers.carry = saved.carry;
      continue;
    }
    if (instruction.opcode === "jr" || instruction.opcode === "jp") {
      const condition =
        instruction.args.length === 2 ? instruction.args[0] : null;
      if (!condition || callbackCondition(condition, registers)) {
        const target = instructionTarget(block, instruction, blocks);
        if (!target || !blocks.has(target)) {
          throw new Error(
            `sprite callback has unresolved branch at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
          );
        }
        blockId = target;
        instructionIndex = 0;
      }
      continue;
    }
    if (instruction.opcode === "ret") {
      if (
        instruction.args.length === 0 ||
        callbackCondition(instruction.args[0], registers)
      ) {
        return effects;
      }
      continue;
    }
    if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
      const target = instructionTarget(block, instruction, blocks);
      if (target === "BattleAnim_Sine_e") {
        registers.e = 0;
        continue;
      }
      if (target === "PlaySFX") continue;
      const outerHelper = target
        ? certifyOuterIncrementHelper(target, blocks)
        : null;
      if (!target || !outerHelper) {
        throw new Error(
          `unsupported sprite callback host call ${String(target)} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      const before = outerMemory.get(outerHelper.symbol) ?? 0;
      outerMemory.set(outerHelper.symbol, (before + 1) & 0xff);
      effects.push({
        from_callback_index: fromCallbackIndex,
        to_callback_index: memory.jumptable_index,
        symbol: outerHelper.symbol,
        operation: "increment",
        source_span: instruction.source_span,
        helper_source_span: outerHelper.sourceSpan,
      });
      continue;
    }
    fail();
  }
  throw new Error(`sprite callback ${entry} exceeds the exact step bound`);
};

const certifySpriteMathHost = (
  target: string,
  sourceSpan: RuntimePresentationSourceSpan,
  blocks: Map<string, ParsedAsmBlock>,
): Extract<
  Extract<
    RuntimePresentationSpriteProgram["callback"],
    { kind: "direct" }
  >["host_operations"][number],
  { op: "sine" | "cosine" }
> => {
  const operation =
    target === "AnimSeqs_Sine"
      ? ("sine" as const)
      : target === "AnimSeqs_Cosine"
        ? ("cosine" as const)
        : null;
  if (!operation) {
    throw new Error(
      `unsupported direct sprite callback host ${target} at ${sourceSpan.file}:${sourceSpan.start_line}`,
    );
  }
  const wrapper = blocks.get(target);
  if (!wrapper) {
    throw new Error(`sprite math host wrapper ${target} is missing`);
  }
  const instructions = wrapper.instructions;
  if (
    instructions.length !== 2 ||
    instructions[0].opcode !== "call" ||
    instructions[1].opcode !== "ret"
  ) {
    throw new Error(`sprite math host wrapper ${target} is malformed`);
  }
  const expectedImplementation =
    operation === "sine" ? "Sprites_Sine" : "Sprites_Cosine";
  const implementation = instructionTarget(wrapper, instructions[0], blocks);
  if (implementation !== expectedImplementation) {
    throw new Error(
      `sprite math host ${target} does not call exact ${expectedImplementation}`,
    );
  }
  const implementationBlock = blocks.get(implementation);
  const sineBlock = blocks.get("Sprites_Sine");
  if (!implementationBlock || !sineBlock) {
    throw new Error(`sprite math implementation ${implementation} is missing`);
  }
  if (
    sineBlock.instructions.length !== 1 ||
    sineBlock.instructions[0].opcode !== "calc_sine_wave"
  ) {
    throw new Error("sprite sine implementation is not exact calc_sine_wave");
  }
  if (
    operation === "cosine" &&
    (implementationBlock.instructions.length !== 1 ||
      implementationBlock.instructions[0].opcode !== "add" ||
      evaluateAsmInteger(
        implementationBlock.instructions[0].args[0],
        new Map(),
      ) !== 0b010000 ||
      implementationBlock.nextBlock !== "Sprites_Sine")
  ) {
    throw new Error(
      "sprite cosine implementation is not exact quarter-wave fallthrough",
    );
  }
  return {
    op: operation,
    target,
    inputs: ["a", "d"],
    output: "a",
    source_span: sourceSpan,
    wrapper_source_span: parsedBlockSourceSpan(wrapper),
    implementation_source_span: {
      file: implementationBlock.file,
      start_line: implementationBlock.startLine,
      end_line: parsedBlockSourceSpan(sineBlock).end_line,
    },
  };
};

type WramByteAliasResolution = {
  sourceSymbol: string;
  canonicalSymbol: string;
  sourceSpans: RuntimePresentationSourceSpan[];
};

const wramDirectiveWidth = (
  value: string,
  constants: ReadonlyMap<string, number>,
): number | null => {
  const [opcode = "", ...tail] = value.split(/\s+/);
  const args = splitInstructionArgs(tail.join(" "));
  if (opcode === "db") return Math.max(args.length, 1);
  if (opcode === "dw") return Math.max(args.length, 1) * 2;
  if (opcode === "ds" && args.length >= 1) {
    return evaluateAsmInteger(args[0], constants);
  }
  return null;
};

const resolveWramByteAlias = (
  symbol: string,
  loadSource: (relativePath: string) => LoadedSource,
  constants: ReadonlyMap<string, number>,
): WramByteAliasResolution => {
  const source = loadSource("ram/wram.asm");
  const declarationPattern = new RegExp(
    `^${symbol.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}::?(?:\\s|$)`,
  );
  const declarationLines = source.lines.flatMap((line, index) =>
    declarationPattern.test(normalizeAsmLine(line)) ? [index] : [],
  );
  if (declarationLines.length !== 1) {
    throw new Error(
      `sprite callback outer memory ${symbol} has no unique WRAM declaration or alias`,
    );
  }
  const declarationLine = declarationLines[0];
  let unionStart: number | null = null;
  for (let index = 0; index <= declarationLine; index += 1) {
    const normalized = normalizeAsmLine(source.lines[index]);
    if (normalized === "UNION") unionStart = index;
    if (normalized === "ENDU") unionStart = null;
  }
  const declarationSpan = {
    file: source.file,
    start_line: declarationLine + 1,
    end_line: declarationLine + 1,
  };
  if (unionStart === null) {
    return {
      sourceSymbol: symbol,
      canonicalSymbol: symbol,
      sourceSpans: [declarationSpan],
    };
  }
  const unionEnd = source.lines.findIndex(
    (line, index) => index > unionStart! && normalizeAsmLine(line) === "ENDU",
  );
  if (unionEnd < 0 || declarationLine >= unionEnd) {
    throw new Error(`WRAM alias union for ${symbol} is unterminated`);
  }
  let variantStart = unionStart + 1;
  for (let index = unionStart + 1; index < declarationLine; index += 1) {
    if (normalizeAsmLine(source.lines[index]) === "NEXTU") {
      variantStart = index + 1;
    }
  }
  let targetOffset = 0;
  for (let index = variantStart; index <= declarationLine; index += 1) {
    const normalized = normalizeAsmLine(source.lines[index]);
    const label = normalized.match(/^([A-Za-z_][A-Za-z0-9_]*)::?\s*(.*)$/);
    if (label?.[1] === symbol) break;
    const directive = label ? label[2] : normalized;
    const width = wramDirectiveWidth(directive, constants);
    if (width !== null) targetOffset += width;
  }

  const aliases: Array<{
    symbol: string;
    sourceSpan: RuntimePresentationSourceSpan;
  }> = [];
  let offset = 0;
  for (let index = unionStart + 1; index < unionEnd; index += 1) {
    const normalized = normalizeAsmLine(source.lines[index]);
    if (!normalized) continue;
    if (normalized === "NEXTU") {
      offset = 0;
      continue;
    }
    const label = normalized.match(/^([A-Za-z_][A-Za-z0-9_]*)::?\s*(.*)$/);
    if (label && offset === targetOffset) {
      aliases.push({
        symbol: label[1],
        sourceSpan: {
          file: source.file,
          start_line: index + 1,
          end_line: index + 1,
        },
      });
    }
    const directive = label ? label[2] : normalized;
    const width = wramDirectiveWidth(directive, constants);
    if (width !== null) offset += width;
  }
  const canonical = aliases[0];
  const sourceAlias = aliases.find((candidate) => candidate.symbol === symbol);
  if (!canonical || !sourceAlias) {
    throw new Error(
      `sprite callback outer memory ${symbol} has no source-proven WRAM alias byte`,
    );
  }
  return {
    sourceSymbol: symbol,
    canonicalSymbol: canonical.symbol,
    sourceSpans: aliases.map((alias) => alias.sourceSpan),
  };
};

const compileDirectSpriteCallback = (
  callback: RuntimePresentationNamedByte,
  dispatchRow: RuntimePresentationAsmInstruction,
  target: string,
  targetBlock: ParsedAsmBlock,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): {
  callback: Extract<
    RuntimePresentationSpriteProgram["callback"],
    { kind: "direct" }
  >;
  dataResources: RuntimePresentationSpriteProgram["callback_data_resources"];
  effects: CallbackOuterEffect[];
} => {
  const family = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === targetBlock.file &&
        candidate.globalLabel === targetBlock.globalLabel,
    )
    .sort((left, right) => left.startLine - right.startLine);
  const instructions = family.flatMap((candidate) => candidate.instructions);
  let property:
    keyof RuntimePresentationSpriteProgram["initial_memory"] | null = null;
  const deltas: Partial<
    Record<keyof RuntimePresentationSpriteProgram["initial_memory"], number>
  > = {};
  const stack: string[] = [];
  const hostOperations: Extract<
    RuntimePresentationSpriteProgram["callback"],
    { kind: "direct" }
  >["host_operations"] = [];
  const outerMemoryReads: Extract<
    RuntimePresentationSpriteProgram["callback"],
    { kind: "direct" }
  >["outer_memory_reads"] = [];
  const framesetReinitializations: Extract<
    RuntimePresentationSpriteProgram["callback"],
    { kind: "direct" }
  >["frameset_reinitializations"] = [];
  let aConstantSymbol: string | null = null;
  let pendingOuterRead:
    (RuntimePresentationOuterMemoryRead & { tested: boolean }) | null = null;
  let dominatingOuterGuard:
    | Extract<
        RuntimePresentationSpriteProgram["callback"],
        { kind: "direct" }
      >["outer_memory_reads"][number]
    | null = null;
  let pendingStructControlRead:
    keyof RuntimePresentationSpriteProgram["initial_memory"] | null = null;
  let aStructSource: {
    property: keyof RuntimePresentationSpriteProgram["initial_memory"];
    delta: number;
  } | null = null;
  const structControlProperties = new Set<
    keyof RuntimePresentationSpriteProgram["initial_memory"]
  >();
  for (const instruction of instructions) {
    const fail = (): never => {
      throw new Error(
        `unsupported direct sprite callback opcode ${instruction.opcode} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    };
    if (instruction.opcode === "ld") {
      const [destination, source] = instruction.args;
      if (destination === "hl" && source) {
        property = spriteMemoryProperty(source);
        if (!property) fail();
        continue;
      }
      const outerRead = source?.match(/^\[([A-Za-z0-9_]+)\]$/);
      if (destination === "a" && outerRead && outerRead[1] !== "hl") {
        const alias = resolveWramByteAlias(outerRead[1], loadSource, constants);
        pendingOuterRead = {
          source_symbol: alias.sourceSymbol,
          symbol: alias.canonicalSymbol,
          predicate: "nonzero",
          comparison_value: null,
          source_span: instruction.source_span,
          alias_source_spans: alias.sourceSpans,
          tested: false,
        };
        aConstantSymbol = null;
        aStructSource = null;
        continue;
      }
      if (destination === "a" && source && source !== "[hl]") {
        evaluateKnownByte(source, constants);
        aConstantSymbol = source;
        aStructSource = null;
        continue;
      }
      if (destination === "d" && source && source !== "[hl]") {
        evaluateKnownByte(source, constants);
        continue;
      }
      if (
        ((destination === "a" || destination === "d") &&
          source === "[hl]" &&
          property) ||
        (destination === "[hl]" &&
          !!source &&
          property &&
          (source === "a" ||
            Number.isInteger(evaluateKnownByte(source, constants))))
      ) {
        if (destination === "a") {
          aConstantSymbol = null;
          pendingStructControlRead = property;
          aStructSource = { property, delta: 0 };
        }
        if (
          destination === "[hl]" &&
          source === "a" &&
          aStructSource?.property === property
        ) {
          deltas[property] =
            ((deltas[property] ?? 0) + aStructSource.delta) & 0xff;
          structControlProperties.add(property);
        }
        continue;
      }
      fail();
    }
    if (
      instruction.opcode === "add" &&
      instruction.args[0] === "hl" &&
      instruction.args[1] === "bc" &&
      property
    ) {
      continue;
    }
    if (
      instruction.opcode === "add" &&
      instruction.args.length === 1 &&
      Number.isInteger(evaluateKnownByte(instruction.args[0], constants))
    ) {
      aConstantSymbol = null;
      if (aStructSource) {
        aStructSource.delta =
          (aStructSource.delta +
            evaluateKnownByte(instruction.args[0], constants)) &
          0xff;
      }
      continue;
    }
    if (
      instruction.opcode === "cp" &&
      instruction.args.length === 1 &&
      Number.isInteger(evaluateKnownByte(instruction.args[0], constants))
    ) {
      if (pendingStructControlRead) {
        structControlProperties.add(pendingStructControlRead);
      }
      if (pendingOuterRead) {
        pendingOuterRead.predicate = "equals";
        pendingOuterRead.comparison_value = evaluateKnownByte(
          instruction.args[0],
          constants,
        );
        pendingOuterRead.tested = true;
      }
      continue;
    }
    if (
      instruction.opcode === "xor" &&
      instruction.args.length === 1 &&
      Number.isInteger(evaluateKnownByte(instruction.args[0], constants))
    ) {
      aConstantSymbol = null;
      aStructSource = null;
      continue;
    }
    if (
      instruction.opcode === "and" &&
      instruction.args.length === 1 &&
      instruction.args[0] === "a"
    ) {
      if (pendingOuterRead) pendingOuterRead.tested = true;
      continue;
    }
    if (
      instruction.opcode === "inc" &&
      instruction.args.length === 1 &&
      instruction.args[0] === "a"
    ) {
      aConstantSymbol = null;
      aStructSource = null;
      continue;
    }
    if (
      instruction.opcode === "inc" &&
      instruction.args[0] === "[hl]" &&
      property
    ) {
      deltas[property] = ((deltas[property] ?? 0) + 1) & 0xff;
      continue;
    }
    if (
      (instruction.opcode === "push" || instruction.opcode === "pop") &&
      (instruction.args[0] === "af" || instruction.args[0] === "de")
    ) {
      if (instruction.opcode === "push") {
        stack.push(instruction.args[0]);
      } else if (stack.pop() !== instruction.args[0]) {
        throw new Error(
          `direct sprite callback stack mismatch at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      continue;
    }
    if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
      const host = instructionTarget(targetBlock, instruction, blocks);
      if (!host) fail();
      if (host === "_ReinitSpriteAnimFrame") {
        if (!aConstantSymbol) {
          throw new Error(
            `sprite frameset reinitializer has no exact frameset operand at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
          );
        }
        let frameset: RuntimePresentationNamedByte;
        try {
          frameset = instructionNamedByte(aConstantSymbol, constants);
        } catch (error) {
          throw new Error(
            `sprite frameset mapping ${aConstantSymbol} is unresolved: ${String(error)}`,
          );
        }
        const implementation = blocks.get("_ReinitSpriteAnimFrame");
        if (!implementation) {
          throw new Error(
            "sprite frameset reinitializer implementation is missing",
          );
        }
        framesetReinitializations.push({
          frameset,
          guard: dominatingOuterGuard,
          application: "every_reachable_scheduler_tick",
          source_span: instruction.source_span,
          implementation_source_span: parsedBlockSourceSpan(implementation),
          reachable_scheduler_ticks: [],
        });
        aConstantSymbol = null;
        continue;
      }
      hostOperations.push(
        certifySpriteMathHost(host!, instruction.source_span, blocks),
      );
      aConstantSymbol = null;
      continue;
    }
    if (
      (instruction.opcode === "jr" || instruction.opcode === "jp") &&
      instruction.args.length === 2 &&
      ["z", "nz", "c", "nc"].includes(instruction.args[0]) &&
      instructionTarget(targetBlock, instruction, blocks)
    ) {
      const condition = instruction.args[0];
      const continuesUnderPredicate =
        pendingOuterRead?.predicate === "nonzero"
          ? condition === "nz"
          : pendingOuterRead?.predicate === "equals"
            ? condition === "z"
            : false;
      if (pendingOuterRead?.tested && continuesUnderPredicate) {
        const { tested: _tested, ...guard } = pendingOuterRead;
        outerMemoryReads.push(guard);
        dominatingOuterGuard = guard;
        pendingOuterRead = null;
      }
      continue;
    }
    if (instruction.opcode === "ret" && instruction.args.length === 1) {
      const condition = instruction.args[0];
      const fallsThroughUnderPredicate =
        pendingOuterRead?.predicate === "nonzero"
          ? condition === "z"
          : pendingOuterRead?.predicate === "equals"
            ? condition === "nz"
            : false;
      if (pendingOuterRead?.tested && fallsThroughUnderPredicate) {
        const { tested: _tested, ...guard } = pendingOuterRead;
        outerMemoryReads.push(guard);
        dominatingOuterGuard = guard;
        pendingOuterRead = null;
      }
      continue;
    }
    if (instruction.opcode === "ret" && instruction.args.length === 0) {
      continue;
    }
    fail();
  }
  if (stack.length !== 0 || instructions.at(-1)?.opcode !== "ret") {
    throw new Error(`direct sprite callback ${target} is not exactly balanced`);
  }
  if (pendingOuterRead) {
    throw new Error(
      `direct sprite callback outer read ${pendingOuterRead.symbol} has no exact predicate`,
    );
  }
  return {
    callback: {
      ...callback,
      kind: "direct",
      table_source_span: dispatchRow.source_span,
      target,
      target_source_span: parsedBlockSourceSpan(targetBlock),
      instructions,
      per_tick_struct_deltas: deltas,
      host_operations: hostOperations,
      outer_memory_reads: outerMemoryReads,
      frameset_reinitializations: framesetReinitializations,
      struct_control_byte_domains: [...structControlProperties].map(
        (controlProperty) => ({
          property: controlProperty,
          initialized_value: 0,
          minimum: 0,
          maximum: 0,
          values: [0],
        }),
      ),
    },
    dataResources: [],
    effects: [],
  };
};

const compileSpriteCallback = (
  callback: RuntimePresentationNamedByte,
  initialMemory: RuntimePresentationSpriteProgram["initial_memory"],
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): {
  callback: RuntimePresentationSpriteProgram["callback"];
  dataResources: RuntimePresentationSpriteProgram["callback_data_resources"];
  effects: CallbackOuterEffect[];
} => {
  const dispatchTable = blocks.get(".Jumptable@DoSpriteAnimFrame");
  if (!dispatchTable) {
    throw new Error("sprite function mapping table is missing");
  }
  const dispatchRows = dispatchTable.instructions.filter(
    (instruction) => instruction.opcode === "dw",
  );
  const dispatchRow = dispatchRows[callback.value];
  if (!dispatchRow || dispatchRow.args.length !== 1) {
    throw new Error(
      `sprite function mapping ${callback.symbol}=${callback.value} is missing`,
    );
  }
  const wrapper = resolveControlTarget(
    dispatchTable,
    dispatchRow.args[0],
    blocks,
    dispatchRow.source_span.start_line,
  );
  const wrapperBlock = blocks.get(wrapper);
  if (!wrapperBlock) {
    throw new Error(
      `sprite function mapping ${callback.symbol} points to missing ${wrapper}`,
    );
  }
  const wrapperInstructions = wrapperBlock.instructions;
  if (
    wrapperInstructions.length !== 2 ||
    !["call", "callfar", "farcall"].includes(wrapperInstructions[0].opcode) ||
    wrapperInstructions[1].opcode !== "ret"
  ) {
    return compileDirectSpriteCallback(
      callback,
      dispatchRow,
      wrapper,
      wrapperBlock,
      blocks,
      constants,
      loadSource,
    );
  }
  const callbackTarget = instructionTarget(
    wrapperBlock,
    wrapperInstructions[0],
    blocks,
  );
  if (!callbackTarget || !blocks.has(callbackTarget)) {
    throw new Error(
      `sprite function wrapper ${wrapper} has missing callback target ${String(callbackTarget)}`,
    );
  }
  const callbackEntry = blocks.get(callbackTarget)!;
  const dispatchIndex = callbackEntry.instructions.findIndex(
    (instruction) =>
      ["jp", "jr"].includes(instruction.opcode) &&
      instruction.args.length === 1 &&
      instruction.args[0] === "hl",
  );
  if (dispatchIndex < 0) {
    throw new Error(
      `sprite callback ${callbackTarget} has no exact local dispatch`,
    );
  }
  const stateTableName = precedingTableSymbol(callbackEntry, dispatchIndex);
  if (!stateTableName) {
    throw new Error(
      `sprite callback ${callbackTarget} has no exact state table`,
    );
  }
  const stateTable = blocks.get(stateTableName);
  if (!stateTable) {
    throw new Error(`sprite callback state table ${stateTableName} is missing`);
  }
  const entries = tableEntries(blocks, stateTableName);
  if (entries.length === 0) {
    throw new Error(`sprite callback state table ${stateTableName} is empty`);
  }
  const stateRows = stateTable.instructions.filter(
    (instruction) => instruction.opcode === "dw",
  );
  if (stateRows.length !== entries.length) {
    throw new Error(
      `sprite callback state table ${stateTableName} is malformed`,
    );
  }
  const compiledEntries = entries.map((target, index) => {
    const block = blocks.get(target);
    if (!block) {
      throw new Error(
        `sprite callback state ${index} points to missing ${target}`,
      );
    }
    const family = [...blocks.values()]
      .filter(
        (candidate) =>
          candidate.file === block.file &&
          candidate.globalLabel === block.globalLabel,
      )
      .sort((left, right) => left.startLine - right.startLine);
    return {
      index,
      target,
      source_span: parsedBlockSourceSpan(block),
      instructions: family.flatMap((candidate) => candidate.instructions),
    };
  });

  const callbackDataSymbols = new Set<string>();
  for (const entry of compiledEntries) {
    for (const instruction of entry.instructions) {
      if (
        instruction.opcode !== "ld" ||
        instruction.args[0] !== "hl" ||
        !instruction.args[1]
      ) {
        continue;
      }
      const owner = [...blocks.values()].find((candidate) =>
        candidate.instructions.includes(instruction),
      );
      if (!owner) {
        throw new Error(
          `sprite callback instruction owner is missing at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      const target = resolveControlTarget(
        owner,
        instruction.args[1],
        blocks,
        instruction.source_span.start_line,
      );
      if (
        blocks
          .get(target)
          ?.instructions.some(
            (candidate) => candidate.opcode.toUpperCase() === "INCLUDE",
          )
      ) {
        callbackDataSymbols.add(target);
      }
    }
  }
  const dataResources = [...callbackDataSymbols].map((symbol) =>
    parseRgb555IncludedResource(symbol, blocks, constants, loadSource),
  );
  const dataResourceMap = new Map(
    dataResources.map((resource) => [resource.symbol, resource]),
  );

  const memory = { ...initialMemory };
  const outerMemory = new Map<string, number>();
  const domain = new Set<number>();
  const effects: CallbackOuterEffect[] = [];
  for (let tick = 0; tick < 1024; tick += 1) {
    const state = memory.jumptable_index;
    if (state >= entries.length) {
      throw new Error(
        `sprite callback ${callbackTarget} reaches index ${state} outside ${entries.length} entries`,
      );
    }
    domain.add(state);
    const tickEffects = executeSpriteCallbackTick(
      entries[state],
      state,
      memory,
      outerMemory,
      blocks,
      constants,
      dataResourceMap,
    );
    effects.push(...tickEffects);
    if (state === entries.length - 1 && memory.jumptable_index === state) {
      break;
    }
    if (tick === 1023) {
      throw new Error(
        `sprite callback ${callbackTarget} does not reach a terminal state`,
      );
    }
  }
  const values = sortedDomain(domain);
  if (
    values.length !== entries.length ||
    values.some((value, index) => value !== index)
  ) {
    throw new Error(
      `sprite callback ${callbackTarget} does not prove every state 0..${entries.length - 1}; reached ${values.join(",")}`,
    );
  }
  return {
    callback: {
      ...callback,
      kind: "state_table",
      table_source_span: dispatchRow.source_span,
      wrapper,
      wrapper_source_span: parsedBlockSourceSpan(wrapperBlock),
      target: callbackTarget,
      target_source_span: parsedBlockSourceSpan(callbackEntry),
      state_table: {
        table: stateTableName,
        source_span: parsedBlockSourceSpan(stateTable),
        entries: compiledEntries,
        index_domain: {
          minimum: values[0],
          maximum: values.at(-1)!,
          values,
        },
      },
    },
    dataResources,
    effects,
  };
};

const compileInitializedSpriteProgram = (
  initialized: InitializedSpriteInstance,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): RuntimePresentationSpriteProgram => {
  const object = parseSpriteObjectRecord(
    initialized.objectSymbol,
    initialized.objectValue,
    blocks,
    constants,
  );
  const compiledFrameset = initialized.framesetOverrideSymbol
    ? instructionNamedByte(initialized.framesetOverrideSymbol, constants)
    : object.frameset;
  const initialMemory = {
    ...initialized.memory,
    frameset_id: compiledFrameset.value,
    anim_seq_id: object.callback.value,
    tile_id: initialized.memory.tile_id,
  };
  const frameset = parseSpriteFrameset(
    compiledFrameset,
    blocks,
    constants,
    loadSource,
  );
  const oamResources = parseSpriteOamResources(frameset.frames, blocks);
  const compiledCallback = compileSpriteCallback(
    object.callback,
    initialMemory,
    blocks,
    constants,
    loadSource,
  );
  const framesetVariants =
    compiledCallback.callback.kind === "direct"
      ? compiledCallback.callback.frameset_reinitializations.map((operation) =>
          parseSpriteFrameset(
            operation.frameset,
            blocks,
            constants,
            loadSource,
          ),
        )
      : [];
  const allOamResources = [
    ...oamResources,
    ...framesetVariants.flatMap((variant) =>
      parseSpriteOamResources(variant.frames, blocks),
    ),
  ].filter(
    (resource, index, resources) =>
      resources.findIndex(
        (candidate) => candidate.oam_set.value === resource.oam_set.value,
      ) === index,
  );
  const schedulerTicks =
    frameset.terminal.op === "delete"
      ? frameset.frames.reduce(
          (ticks, frame) => ticks + frame.duration + 1,
          frameset.waits.reduce((ticks, wait) => ticks + wait.duration + 1, 1),
        )
      : null;
  return {
    instance: initialized.instance,
    struct_slot: initialized.structSlot,
    initializer_source_span: initialized.sourceSpan,
    allocation_source_span: initialized.allocationSourceSpan,
    object: object.object,
    initial_memory: initialMemory,
    frameset,
    frameset_variants: framesetVariants,
    callback: compiledCallback.callback,
    callback_data_resources: compiledCallback.dataResources,
    dictionary: object.dictionary,
    oam_resources: allOamResources,
    outer_state_effects: compiledCallback.effects,
    lifetime: {
      allocation_dispatcher_entry: null,
      allocation_dispatch_tick: initialized.allocationDispatchTick,
      active_dispatcher_entries: [],
      scheduler_ticks: schedulerTicks,
      deinitialized_after_dispatch_tick:
        schedulerTicks === null
          ? null
          : initialized.allocationDispatchTick + schedulerTicks - 1,
      callback_before_frame_update: true,
      outer_byte_domains: [],
      deinitializer: null,
      outer_scene_advances: [],
      pre_scheduler_waits: [],
      handler_host_operations: [],
    },
  };
};

type SpriteSchedulerBridge = {
  actions: Map<
    number,
    Array<{ kind: "keep" | "increment" | "set_bit"; bit?: number }>
  >;
  operations: RuntimePresentationSpriteOperation[];
  programs: RuntimePresentationSpriteProgram[];
};

type SpriteSchedulerLoopSite = {
  callSite: { block: ParsedAsmBlock; index: number };
  scheduler: RuntimePresentationAsmInstruction;
  delay: RuntimePresentationAsmInstruction;
};

const findSpriteSchedulerLoopSite = (
  dispatcher: ParsedAsmBlock,
  blocks: Map<string, ParsedAsmBlock>,
): SpriteSchedulerLoopSite | null => {
  const callSites = [...blocks.values()].flatMap((candidate) =>
    candidate.instructions.flatMap((instruction, index) =>
      instructionTarget(candidate, instruction, blocks) ===
      dispatcher.globalLabel
        ? [{ block: candidate, index }]
        : [],
    ),
  );
  if (callSites.length !== 1) return null;
  const callSite = callSites[0];
  const afterDispatch = callSite.block.instructions.slice(callSite.index + 1);
  const scheduler = afterDispatch[0];
  const delay = afterDispatch[1];
  const schedulerTarget = scheduler
    ? instructionTarget(callSite.block, scheduler, blocks)
    : null;
  const delayTarget = delay
    ? instructionTarget(callSite.block, delay, blocks)
    : null;
  const hasSpriteSchedulerElsewhere = afterDispatch.some(
    (instruction) =>
      instructionTarget(callSite.block, instruction, blocks) ===
      "PlaySpriteAnimations",
  );
  if (
    schedulerTarget !== "PlaySpriteAnimations" ||
    delayTarget !== "DelayFrame"
  ) {
    if (hasSpriteSchedulerElsewhere) {
      throw new Error(
        "sprite scheduler order must be dispatcher -> PlaySpriteAnimations -> DelayFrame",
      );
    }
    return null;
  }
  return { callSite, scheduler, delay };
};

type DeferredSpriteInitializer = {
  entryIndex: number;
  entry: string;
  callSite: RuntimePresentationSourceSpan;
  initializer: RuntimePresentationSourceSpan;
};

const findPotentialSpriteInitializer = (
  blockId: string,
  blocks: Map<string, ParsedAsmBlock>,
  visited: Set<string>,
  rootCallSite: RuntimePresentationSourceSpan | null = null,
): Omit<DeferredSpriteInitializer, "entryIndex" | "entry"> | null => {
  if (visited.has(blockId)) return null;
  const block = blocks.get(blockId);
  if (!block) return null;
  const path = new Set(visited).add(blockId);

  for (const instruction of block.instructions) {
    const target = instructionTarget(block, instruction, blocks);
    const isCall = ["call", "callfar", "farcall"].includes(instruction.opcode);
    const isBranch = ["jp", "jr"].includes(instruction.opcode);
    if (target === "InitSpriteAnimStruct") {
      return {
        callSite: rootCallSite ?? instruction.source_span,
        initializer: instruction.source_span,
      };
    }
    if (target && blocks.has(target)) {
      const nested = findPotentialSpriteInitializer(
        target,
        blocks,
        new Set(path),
        isCall ? (rootCallSite ?? instruction.source_span) : rootCallSite,
      );
      if (nested) return nested;
    }
    if (isBranch && instruction.args.length === 1) return null;
    if (instruction.opcode === "ret" && instruction.args.length === 0) {
      return null;
    }
  }
  if (!block.nextBlock) return null;
  return findPotentialSpriteInitializer(
    block.nextBlock,
    blocks,
    path,
    rootCallSite,
  );
};

type CounterDrivenAllocationCall = {
  target: string;
  objectSymbol: string | null;
  sourceSpan: RuntimePresentationSourceSpan;
  dispatchTick: number;
  x: number;
  y: number;
};

const handlerFamily = (
  entry: string,
  blocks: Map<string, ParsedAsmBlock>,
): ParsedAsmBlock[] => {
  const block = blocks.get(entry);
  if (!block) return [];
  return [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === block.file &&
        candidate.globalLabel === block.globalLabel,
    )
    .sort((left, right) => left.startLine - right.startLine);
};

const counterDrivenHandlerSymbol = (
  entry: string,
  blocks: Map<string, ParsedAsmBlock>,
): string | null => {
  const instructions = handlerFamily(entry, blocks).flatMap(
    (block) => block.instructions,
  );
  for (let index = 0; index + 2 < instructions.length; index += 1) {
    const [loadPointer, loadValue, increment] = instructions.slice(
      index,
      index + 3,
    );
    if (
      loadPointer.opcode === "ld" &&
      loadPointer.args[0] === "hl" &&
      /^w[A-Za-z0-9_]+$/.test(loadPointer.args[1] ?? "") &&
      loadValue.opcode === "ld" &&
      loadValue.args[0] === "a" &&
      loadValue.args[1] === "[hl]" &&
      increment.opcode === "inc" &&
      increment.args[0] === "[hl]"
    ) {
      return loadPointer.args[1];
    }
  }
  return null;
};

const previousHandlerInitializesCounter = (
  entries: string[],
  entryIndex: number,
  counterSymbol: string,
  blocks: Map<string, ParsedAsmBlock>,
): boolean =>
  entryIndex > 0 &&
  instructionsInitializeStateToZero(
    handlerFamily(entries[entryIndex - 1], blocks).flatMap(
      (block) => block.instructions,
    ),
    counterSymbol,
  );

const previousHandlerClearsSpriteInstances = (
  entries: string[],
  entryIndex: number,
  blocks: Map<string, ParsedAsmBlock>,
): boolean =>
  entryIndex > 0 &&
  handlerFamily(entries[entryIndex - 1], blocks).some((block) =>
    block.instructions.some(
      (instruction) =>
        instructionTarget(block, instruction, blocks) === "ClearSpriteAnims",
    ),
  );

const runCounterDrivenHandlerTick = (
  entry: string,
  counterSymbol: string,
  counter: number,
  dispatchTick: number,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): {
  counter: number;
  allocations: CounterDrivenAllocationCall[];
  nextScene: boolean;
  hostCalls: Array<{
    target: string;
    sourceSpan: RuntimePresentationSourceSpan;
  }>;
} => {
  let blockId = entry;
  let instructionIndex = 0;
  let a: number | null = 0;
  let aSymbol: string | null = null;
  let d = 0;
  let e = 0;
  let hl: string | null = null;
  let zero: boolean | null = null;
  let carry: boolean | null = null;
  const memory = new Map<string, number>([[counterSymbol, counter]]);
  const afStack: Array<{
    a: number | null;
    aSymbol: string | null;
    zero: boolean | null;
    carry: boolean | null;
  }> = [];
  const allocations: CounterDrivenAllocationCall[] = [];
  const hostCalls: Array<{
    target: string;
    sourceSpan: RuntimePresentationSourceSpan;
  }> = [];
  const requireA = (instruction: RuntimePresentationAsmInstruction): number => {
    if (a === null) {
      throw new Error(
        `counter-driven sprite handler reads an opaque A at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    }
    return a;
  };
  for (let steps = 0; steps < 500; steps += 1) {
    const block = blocks.get(blockId);
    if (!block) throw new Error(`sprite handler block ${blockId} is missing`);
    if (instructionIndex >= block.instructions.length) {
      if (!block.nextBlock) {
        throw new Error(`sprite handler ${blockId} falls out of source`);
      }
      blockId = block.nextBlock;
      instructionIndex = 0;
      continue;
    }
    const instruction = block.instructions[instructionIndex++];
    const fail = (): never => {
      throw new Error(
        `unsupported counter-driven sprite handler opcode ${instruction.opcode} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    };
    if (instruction.opcode === "ld" || instruction.opcode === "ldh") {
      const [destination, source] = instruction.args;
      if (destination === "hl" && source) {
        hl = source;
        continue;
      }
      if (destination === "a" && source === "[hl]" && hl) {
        a = memory.get(hl) ?? 0;
        aSymbol = null;
        continue;
      }
      if (destination === "a" && source) {
        a = evaluateKnownByte(source, constants);
        aSymbol = source;
        continue;
      }
      if (destination === "[hl]" && source && hl) {
        memory.set(
          hl,
          source === "a"
            ? requireA(instruction)
            : evaluateKnownByte(source, constants),
        );
        continue;
      }
      const directMemory = destination?.match(/^\[([A-Za-z0-9_]+)\]$/);
      if (directMemory && source === "a") {
        memory.set(directMemory[1], requireA(instruction));
        continue;
      }
      if (destination === "de" && source) {
        d = 0;
        e = 0;
        continue;
      }
      fail();
    }
    if (instruction.opcode === "inc" && instruction.args[0] === "[hl]" && hl) {
      const value = ((memory.get(hl) ?? 0) + 1) & 0xff;
      memory.set(hl, value);
      zero = value === 0;
      continue;
    }
    if (instruction.opcode === "inc" && instruction.args[0] === "a") {
      const value = requireA(instruction);
      a = (value + 1) & 0xff;
      zero = a === 0;
      continue;
    }
    if (instruction.opcode === "cp") {
      const value = requireA(instruction);
      const operand = evaluateKnownByte(instruction.args[0], constants);
      zero = value === operand;
      carry = value < operand;
      continue;
    }
    if (instruction.opcode === "xor" && instruction.args[0] === "a") {
      a = 0;
      aSymbol = null;
      zero = true;
      carry = false;
      continue;
    }
    if (instruction.opcode === "depixel") {
      if (instruction.args.length < 2 || instruction.args.length > 4) fail();
      const tileWidth = constants.get("TILE_WIDTH");
      if (tileWidth === undefined) fail();
      d =
        evaluateAsmInteger(instruction.args[0], constants) * tileWidth! +
        (instruction.args[2]
          ? evaluateAsmInteger(instruction.args[2], constants)
          : 0);
      e =
        evaluateAsmInteger(instruction.args[1], constants) * tileWidth! +
        (instruction.args[3]
          ? evaluateAsmInteger(instruction.args[3], constants)
          : 0);
      continue;
    }
    if (instruction.opcode === "push" && instruction.args[0] === "af") {
      afStack.push({ a, aSymbol, zero, carry });
      continue;
    }
    if (instruction.opcode === "pop" && instruction.args[0] === "af") {
      const saved = afStack.pop();
      if (!saved) fail();
      ({ a, aSymbol, zero, carry } = saved!);
      continue;
    }
    if (instruction.opcode === "jr" || instruction.opcode === "jp") {
      const condition =
        instruction.args.length === 2 ? instruction.args[0] : null;
      const take =
        condition === null
          ? true
          : condition === "z"
            ? zero
            : condition === "nz"
              ? zero === null
                ? null
                : !zero
              : condition === "c"
                ? carry
                : condition === "nc"
                  ? carry === null
                    ? null
                    : !carry
                  : null;
      if (take === null) {
        throw new Error(
          `counter-driven sprite handler has an opaque branch at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
      if (take) {
        const target = instructionTarget(block, instruction, blocks);
        if (!target || !blocks.has(target)) fail();
        blockId = target!;
        instructionIndex = 0;
      }
      continue;
    }
    if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
      const target = instructionTarget(block, instruction, blocks);
      const nextScene = target
        ? certifyOuterIncrementHelper(target, blocks)
        : null;
      if (nextScene?.symbol === "wJumptableIndex") {
        return {
          counter: memory.get(counterSymbol) ?? counter,
          allocations,
          nextScene: true,
          hostCalls,
        };
      }
      if (
        target === "InitSpriteAnimStruct" ||
        (target && findPotentialSpriteInitializer(target, blocks, new Set()))
      ) {
        allocations.push({
          target: target!,
          objectSymbol: target === "InitSpriteAnimStruct" ? aSymbol : null,
          sourceSpan: instruction.source_span,
          dispatchTick,
          x: e,
          y: d,
        });
      } else if (target) {
        hostCalls.push({ target, sourceSpan: instruction.source_span });
      }
      a = null;
      aSymbol = null;
      zero = null;
      carry = null;
      continue;
    }
    if (instruction.opcode === "ret" && instruction.args.length === 0) {
      if (afStack.length !== 0) {
        throw new Error(`counter-driven sprite handler ${entry} leaks AF`);
      }
      return {
        counter: memory.get(counterSymbol) ?? counter,
        allocations,
        nextScene: false,
        hostCalls,
      };
    }
    fail();
  }
  throw new Error(`counter-driven sprite handler ${entry} exceeds step bound`);
};

const compileSpriteAllocationHelper = (
  call: CounterDrivenAllocationCall,
  startingAnimationIndex: number,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): {
  instances: InitializedSpriteInstance[];
  nextAnimationIndex: number;
} => {
  const family = handlerFamily(call.target, blocks);
  if (family.length === 0) {
    throw new Error(`sprite allocation helper ${call.target} is missing`);
  }
  const instructions = family.flatMap((block) => block.instructions);
  let aSymbol: string | null = null;
  let property:
    keyof RuntimePresentationSpriteProgram["initial_memory"] | null = null;
  let current: InitializedSpriteInstance | null = null;
  const instances: InitializedSpriteInstance[] = [];
  const deStack: Array<{ x: number; y: number }> = [];
  let x = call.x;
  let y = call.y;
  let nextAnimationIndex = startingAnimationIndex;
  for (const instruction of instructions) {
    const fail = (): never => {
      throw new Error(
        `unsupported sprite allocation helper opcode ${instruction.opcode} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    };
    if (instruction.opcode === "push" && instruction.args[0] === "de") {
      deStack.push({ x, y });
      continue;
    }
    if (instruction.opcode === "pop" && instruction.args[0] === "de") {
      const saved = deStack.pop();
      if (!saved) fail();
      ({ x, y } = saved!);
      continue;
    }
    if (instruction.opcode === "ld") {
      const [destination, source] = instruction.args;
      if (destination === "a" && source) {
        aSymbol = source;
        continue;
      }
      if (destination === "hl" && source) {
        property = spriteMemoryProperty(source);
        if (!property) fail();
        continue;
      }
      if (destination === "[hl]" && source && property && current) {
        current.memory[property] = evaluateKnownByte(source, constants);
        continue;
      }
      fail();
    }
    if (
      instruction.opcode === "add" &&
      instruction.args[0] === "hl" &&
      instruction.args[1] === "bc" &&
      property
    ) {
      continue;
    }
    if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
      const owner =
        family.find((block) => block.instructions.includes(instruction)) ??
        family[0];
      const target = instructionTarget(owner, instruction, blocks);
      if (target === "InitSpriteAnimStruct") {
        if (!aSymbol) fail();
        const objectSymbol = aSymbol!;
        const objectValue = evaluateKnownByte(objectSymbol, constants);
        if (instances.length >= 10) {
          throw new Error("sprite allocation exceeds ten exact struct slots");
        }
        const memory = defaultSpriteMemory();
        memory.index = nextAnimationIndex;
        nextAnimationIndex = (nextAnimationIndex + 1) & 0xff;
        memory.xcoord = x;
        memory.ycoord = y;
        current = {
          instance:
            `sprite:${call.sourceSpan.file}:${call.sourceSpan.start_line}:` +
            `${call.dispatchTick}:${instruction.source_span.start_line}`,
          structSlot: instances.length,
          objectSymbol,
          objectValue,
          sourceSpan: instruction.source_span,
          allocationSourceSpan: call.sourceSpan,
          allocationDispatchTick: call.dispatchTick,
          framesetOverrideSymbol: null,
          memory,
        };
        instances.push(current!);
        property = null;
        continue;
      }
      if (target === "ReinitSpriteAnimFrame") {
        if (!current || !aSymbol) fail();
        const currentInstance = current!;
        const framesetSymbol = aSymbol!;
        currentInstance.framesetOverrideSymbol = framesetSymbol;
        currentInstance.memory.frameset_id = evaluateKnownByte(
          framesetSymbol,
          constants,
        );
        currentInstance.memory.duration = 0;
        currentInstance.memory.frame = 0xff;
        property = null;
        continue;
      }
      fail();
    }
    if (instruction.opcode === "ret" && instruction.args.length === 0) {
      continue;
    }
    fail();
  }
  if (deStack.length !== 0 || instances.length === 0) {
    throw new Error(`sprite allocation helper ${call.target} is unbalanced`);
  }
  if (instances.some((instance) => !instance.framesetOverrideSymbol)) {
    throw new Error(
      `sprite allocation helper ${call.target} has no exact frameset override`,
    );
  }
  return { instances, nextAnimationIndex };
};

const compileDirectSpriteAllocation = (
  call: CounterDrivenAllocationCall,
  animationIndex: number,
  structSlot: number,
  constants: ReadonlyMap<string, number>,
): InitializedSpriteInstance => {
  if (call.target !== "InitSpriteAnimStruct" || !call.objectSymbol) {
    throw new Error(
      `direct sprite allocation at ${call.sourceSpan.file}:${call.sourceSpan.start_line} has no exact object operand`,
    );
  }
  const objectValue = evaluateKnownByte(call.objectSymbol, constants);
  const memory = defaultSpriteMemory();
  memory.index = animationIndex;
  memory.xcoord = call.x;
  memory.ycoord = call.y;
  return {
    instance:
      `sprite:${call.sourceSpan.file}:${call.sourceSpan.start_line}:` +
      `${call.dispatchTick}`,
    structSlot,
    objectSymbol: call.objectSymbol,
    objectValue,
    sourceSpan: call.sourceSpan,
    allocationSourceSpan: call.sourceSpan,
    allocationDispatchTick: call.dispatchTick,
    framesetOverrideSymbol: null,
    memory,
  };
};

const exactHandlerByteInitialization = (
  entry: string,
  symbol: string,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): { value: number; sourceSpan: RuntimePresentationSourceSpan } | null => {
  let a: number | null = null;
  let hl: string | null = null;
  let result: {
    value: number;
    sourceSpan: RuntimePresentationSourceSpan;
  } | null = null;
  for (const instruction of handlerFamily(entry, blocks).flatMap(
    (block) => block.instructions,
  )) {
    if (instruction.opcode === "xor" && instruction.args[0] === "a") {
      a = 0;
      continue;
    }
    if (
      (instruction.opcode === "ld" || instruction.opcode === "ldh") &&
      instruction.args[0] === "a" &&
      instruction.args[1] &&
      !instruction.args[1].startsWith("[")
    ) {
      try {
        a = evaluateKnownByte(instruction.args[1], constants);
      } catch {
        a = null;
      }
      continue;
    }
    if (
      (instruction.opcode === "ld" || instruction.opcode === "ldh") &&
      instruction.args[0] === "hl" &&
      instruction.args[1]
    ) {
      hl = instruction.args[1];
      continue;
    }
    if (
      (instruction.opcode === "ld" || instruction.opcode === "ldh") &&
      instruction.args[1] === "a" &&
      a !== null &&
      (instruction.args[0] === `[${symbol}]` ||
        (instruction.args[0] === "[hl]" && hl === symbol))
    ) {
      result = { value: a, sourceSpan: instruction.source_span };
      continue;
    }
    if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
      a = null;
    }
  }
  return result;
};

const executeDirectSpriteCallbackTick = (
  callback: Extract<
    RuntimePresentationSpriteProgram["callback"],
    { kind: "direct" }
  >,
  memory: RuntimePresentationSpriteProgram["initial_memory"],
  outerMemory: Map<string, number>,
  schedulerTick: number,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): void => {
  let blockId = callback.target;
  let instructionIndex = 0;
  let a = 0;
  let d = 0;
  let zero = false;
  let carry = false;
  let hl:
    | { kind: "struct"; property: keyof typeof memory }
    | { kind: "outer"; symbol: string }
    | null = null;
  const stack: Array<{ a: number; zero: boolean; carry: boolean }> = [];
  const resolveOuterSymbol = (sourceSymbol: string): string =>
    callback.outer_memory_reads.find(
      (read) => read.source_symbol === sourceSymbol,
    )?.symbol ?? sourceSymbol;
  const readHl = (): number => {
    if (!hl) throw new Error("direct sprite callback reads unresolved [hl]");
    return hl.kind === "struct"
      ? memory[hl.property]
      : (() => {
          const value = outerMemory.get(hl.symbol);
          if (value === undefined) {
            throw new Error(
              `sprite callback outer memory ${hl.symbol} is not initialized`,
            );
          }
          return value;
        })();
  };
  const writeHl = (value: number): void => {
    if (!hl) throw new Error("direct sprite callback writes unresolved [hl]");
    if (hl.kind === "struct") memory[hl.property] = value & 0xff;
    else outerMemory.set(hl.symbol, value & 0xff);
  };
  for (let steps = 0; steps < 500; steps += 1) {
    const block = blocks.get(blockId);
    if (!block) throw new Error(`direct sprite callback ${blockId} is missing`);
    if (instructionIndex >= block.instructions.length) {
      if (!block.nextBlock) {
        throw new Error(
          `direct sprite callback ${blockId} falls out of source`,
        );
      }
      blockId = block.nextBlock;
      instructionIndex = 0;
      continue;
    }
    const instruction = block.instructions[instructionIndex++];
    const fail = (): never => {
      throw new Error(
        `unsupported direct sprite callback opcode ${instruction.opcode} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    };
    if (instruction.opcode === "ld") {
      const [destination, source] = instruction.args;
      if (destination === "hl" && source) {
        const property = spriteMemoryProperty(source);
        hl = property
          ? { kind: "struct", property }
          : /^w[A-Za-z0-9_]+$/.test(source)
            ? { kind: "outer", symbol: resolveOuterSymbol(source) }
            : null;
        if (!hl) fail();
        continue;
      }
      const directRead = source?.match(/^\[([A-Za-z0-9_]+)\]$/);
      if (destination === "a" && directRead && directRead[1] !== "hl") {
        const resolvedSymbol = resolveOuterSymbol(directRead[1]);
        const value = outerMemory.get(resolvedSymbol);
        if (value === undefined) {
          throw new Error(
            `sprite callback outer memory ${resolvedSymbol} is not initialized`,
          );
        }
        a = value;
        continue;
      }
      if (destination === "a" && source === "[hl]") {
        a = readHl();
        continue;
      }
      if (destination === "d" && source === "[hl]") {
        d = readHl();
        continue;
      }
      if (destination === "a" && source) {
        a = evaluateKnownByte(source, constants);
        continue;
      }
      if (destination === "d" && source) {
        d = evaluateKnownByte(source, constants);
        continue;
      }
      if (destination === "[hl]" && source) {
        writeHl(source === "a" ? a : evaluateKnownByte(source, constants));
        continue;
      }
      fail();
    }
    if (
      instruction.opcode === "add" &&
      instruction.args[0] === "hl" &&
      instruction.args[1] === "bc" &&
      hl?.kind === "struct"
    ) {
      continue;
    }
    if (instruction.opcode === "add" && instruction.args.length === 1) {
      const operand = evaluateKnownByte(instruction.args[0], constants);
      const sum = a + operand;
      a = sum & 0xff;
      zero = a === 0;
      carry = sum > 0xff;
      continue;
    }
    if (instruction.opcode === "and" && instruction.args[0] === "a") {
      zero = a === 0;
      carry = false;
      continue;
    }
    if (instruction.opcode === "cp" && instruction.args.length === 1) {
      const operand = evaluateKnownByte(instruction.args[0], constants);
      zero = a === operand;
      carry = a < operand;
      continue;
    }
    if (instruction.opcode === "xor" && instruction.args.length === 1) {
      a ^= evaluateKnownByte(instruction.args[0], constants);
      a &= 0xff;
      zero = a === 0;
      carry = false;
      continue;
    }
    if (instruction.opcode === "inc" && instruction.args[0] === "a") {
      a = (a + 1) & 0xff;
      zero = a === 0;
      continue;
    }
    if (instruction.opcode === "inc" && instruction.args[0] === "[hl]") {
      const value = (readHl() + 1) & 0xff;
      writeHl(value);
      zero = value === 0;
      continue;
    }
    if (
      (instruction.opcode === "push" || instruction.opcode === "pop") &&
      ["af", "de"].includes(instruction.args[0])
    ) {
      if (instruction.opcode === "push") {
        stack.push({ a, zero, carry });
      } else {
        const saved = stack.pop();
        if (!saved) fail();
        ({ a, zero, carry } = saved!);
      }
      continue;
    }
    if (instruction.opcode === "jr" || instruction.opcode === "jp") {
      const condition =
        instruction.args.length === 2 ? instruction.args[0] : null;
      const take =
        condition === null
          ? true
          : condition === "z"
            ? zero
            : condition === "nz"
              ? !zero
              : condition === "c"
                ? carry
                : condition === "nc"
                  ? !carry
                  : fail();
      if (take) {
        const target = instructionTarget(block, instruction, blocks);
        if (!target || !blocks.has(target)) fail();
        blockId = target!;
        instructionIndex = 0;
      }
      continue;
    }
    if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
      const target = instructionTarget(block, instruction, blocks);
      if (target === "AnimSeqs_Sine" || target === "AnimSeqs_Cosine") {
        // Exact math is already source-certified; its byte only feeds an offset.
        a = 0;
        continue;
      }
      if (target === "_ReinitSpriteAnimFrame") {
        const operation = callback.frameset_reinitializations.find(
          (candidate) =>
            candidate.source_span.file === instruction.source_span.file &&
            candidate.source_span.start_line ===
              instruction.source_span.start_line,
        );
        if (!operation) fail();
        memory.frameset_id = operation!.frameset.value;
        memory.duration = 0;
        memory.frame = 0xff;
        operation!.reachable_scheduler_ticks.push(schedulerTick);
        continue;
      }
      fail();
    }
    if (instruction.opcode === "ret") {
      const shouldReturn =
        instruction.args.length === 0
          ? true
          : instruction.args[0] === "z"
            ? zero
            : instruction.args[0] === "nz"
              ? !zero
              : instruction.args[0] === "c"
                ? carry
                : instruction.args[0] === "nc"
                  ? !carry
                  : fail();
      if (shouldReturn) {
        if (stack.length !== 0) {
          throw new Error(
            `direct sprite callback ${callback.target} leaks stack`,
          );
        }
        return;
      }
      continue;
    }
    fail();
  }
  throw new Error(
    `direct sprite callback ${callback.target} exceeds step bound`,
  );
};

type PersistentHandlerTick = {
  deinitialized: RuntimePresentationSourceSpan | null;
  nextScene: boolean;
  nextSceneSourceSpan: RuntimePresentationSourceSpan | null;
  preSchedulerWaits: Array<{
    target: "DelayFrames";
    frameCount: number;
    sourceSpan: RuntimePresentationSourceSpan;
    implementationSourceSpan: RuntimePresentationSourceSpan;
  }>;
  hostCalls: Array<{
    target: string;
    sourceSpan: RuntimePresentationSourceSpan;
  }>;
};

const certifyDelayFrames = (
  blocks: Map<string, ParsedAsmBlock>,
): RuntimePresentationSourceSpan => {
  const delayFrame = blocks.get("DelayFrame");
  const delayFrames = blocks.get("DelayFrames");
  if (!delayFrame || !delayFrames || delayFrame.file !== delayFrames.file) {
    throw new Error("typed DelayFrames source implementation is missing");
  }
  const oneFrame = handlerFamily("DelayFrame", blocks).flatMap(
    (block) => block.instructions,
  );
  const manyFrames = handlerFamily("DelayFrames", blocks).flatMap(
    (block) => block.instructions,
  );
  const oneFrameExpected = [
    ["ld", "a", "1"],
    ["ld", "[wVBlankOccurred]", "a"],
    ["halt"],
    ["nop"],
    ["ld", "a", "[wVBlankOccurred]"],
    ["and", "a"],
    ["jr", "nz", ".halt"],
    ["ret"],
  ];
  const manyFramesExpected = [
    ["call", "DelayFrame"],
    ["dec", "c"],
    ["jr", "nz", "DelayFrames"],
    ["ret"],
  ];
  const exact = (
    instructions: RuntimePresentationAsmInstruction[],
    expected: string[][],
  ): boolean =>
    instructions.length === expected.length &&
    instructions.every(
      (instruction, index) =>
        [instruction.opcode, ...instruction.args].join("\0") ===
        expected[index].join("\0"),
    );
  if (
    !exact(oneFrame, oneFrameExpected) ||
    !exact(manyFrames, manyFramesExpected) ||
    instructionTarget(delayFrame, oneFrame[6], blocks) !== ".halt@DelayFrame" ||
    instructionTarget(delayFrames, manyFrames[0], blocks) !== "DelayFrame" ||
    instructionTarget(delayFrames, manyFrames[2], blocks) !== "DelayFrames"
  ) {
    throw new Error(
      "typed DelayFrames must loop through the exact one-VBlank DelayFrame implementation",
    );
  }
  return {
    file: delayFrame.file,
    start_line: delayFrame.startLine,
    end_line: manyFrames.at(-1)!.source_span.end_line,
  };
};

const certifyTrackedStateHost = (
  target: string,
  trackedSymbols: ReadonlySet<string>,
  blocks: Map<string, ParsedAsmBlock>,
  visited: ReadonlySet<string> = new Set(),
): void => {
  if (target === "PlaySFX") return;
  if (visited.has(target)) return;
  const family = handlerFamily(target, blocks);
  if (family.length === 0) {
    throw new Error(`persistent sprite handler host ${target} is unresolved`);
  }
  const path = new Set(visited).add(target);
  let hlSymbol: string | null = null;
  let deSymbol: string | null = null;
  for (const instruction of family.flatMap((block) => block.instructions)) {
    if (
      (instruction.opcode === "ld" || instruction.opcode === "ldh") &&
      instruction.args[0] === "hl" &&
      instruction.args[1]
    ) {
      hlSymbol = instruction.args[1];
      continue;
    }
    if (
      instruction.opcode === "ld" &&
      instruction.args[0] === "de" &&
      instruction.args[1]
    ) {
      deSymbol = instruction.args[1];
      continue;
    }
    const destination = instruction.args[0];
    const direct = destination?.match(/^\[([A-Za-z0-9_]+)\]$/);
    if (direct && trackedSymbols.has(direct[1])) {
      throw new Error(
        `persistent sprite handler host ${target} writes tracked ${direct[1]} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    }
    if (["[hl]", "[hli]", "[hld]"].includes(destination ?? "") && hlSymbol) {
      const base = hlSymbol.match(/^([A-Za-z0-9_]+)/)?.[1];
      if (base && trackedSymbols.has(base)) {
        throw new Error(
          `persistent sprite handler host ${target} writes tracked ${base} through HL at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
        );
      }
    }
    if (!["call", "callfar", "farcall"].includes(instruction.opcode)) {
      continue;
    }
    const owner = family.find((block) =>
      block.instructions.includes(instruction),
    );
    const called = owner ? instructionTarget(owner, instruction, blocks) : null;
    if (
      called === "InitSpriteAnimStruct" ||
      called === "DeinitializeAllSprites"
    ) {
      throw new Error(
        `persistent sprite handler host ${target} has an unmodeled sprite lifetime effect`,
      );
    }
    if (called === "ByteFill") {
      if (!hlSymbol) {
        throw new Error(
          `persistent sprite handler host ${target} calls ByteFill with unresolved HL`,
        );
      }
      const base = hlSymbol.match(/^([A-Za-z0-9_]+)/)?.[1];
      if (!base || trackedSymbols.has(base)) {
        throw new Error(
          `persistent sprite handler host ${target} may fill tracked ${String(base)}`,
        );
      }
      continue;
    }
    if (called === "CopyBytes") {
      const base = deSymbol?.match(/^([A-Za-z0-9_]+)/)?.[1];
      if (!base || trackedSymbols.has(base)) {
        throw new Error(
          `persistent sprite handler host ${target} may copy into tracked ${String(base)}`,
        );
      }
      continue;
    }
    if (!called) {
      throw new Error(
        `persistent sprite handler host ${target} has an unresolved call at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    }
    certifyTrackedStateHost(called, trackedSymbols, blocks, path);
  }
};

const runPersistentHandlerTick = (
  entry: string,
  memory: Map<string, number>,
  trackedSymbols: ReadonlySet<string>,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): PersistentHandlerTick => {
  let blockId = entry;
  let instructionIndex = 0;
  let a: number | null = 0;
  let c: number | null = null;
  let hl: string | null = null;
  let zero: boolean | null = null;
  let carry: boolean | null = null;
  let deinitialized: RuntimePresentationSourceSpan | null = null;
  const hostCalls: PersistentHandlerTick["hostCalls"] = [];
  const preSchedulerWaits: PersistentHandlerTick["preSchedulerWaits"] = [];
  const requireA = (instruction: RuntimePresentationAsmInstruction): number => {
    if (a === null) {
      throw new Error(
        `persistent sprite handler reads opaque A at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    }
    return a;
  };
  for (let steps = 0; steps < 500; steps += 1) {
    const block = blocks.get(blockId);
    if (!block)
      throw new Error(`persistent sprite handler ${blockId} is missing`);
    if (instructionIndex >= block.instructions.length) {
      if (!block.nextBlock) {
        throw new Error(
          `persistent sprite handler ${blockId} falls out of source`,
        );
      }
      blockId = block.nextBlock;
      instructionIndex = 0;
      continue;
    }
    const instruction = block.instructions[instructionIndex++];
    const fail = (): never => {
      throw new Error(
        `unsupported persistent sprite handler opcode ${instruction.opcode} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
      );
    };
    if (instruction.opcode === "ld" || instruction.opcode === "ldh") {
      const [destination, source] = instruction.args;
      if (destination === "hl" && source) {
        hl = source;
        continue;
      }
      if (destination === "a" && source === "[hl]" && hl) {
        const value = memory.get(hl);
        if (value === undefined) fail();
        a = value!;
        continue;
      }
      const directRead = source?.match(/^\[([A-Za-z0-9_]+)\]$/);
      if (destination === "a" && directRead) {
        const value = memory.get(directRead[1]);
        if (value === undefined) fail();
        a = value!;
        continue;
      }
      if (destination === "a" && source) {
        a = source === "c" ? c : evaluateKnownByte(source, constants);
        if (a === null) fail();
        continue;
      }
      if (destination === "c" && source) {
        c =
          source === "a"
            ? requireA(instruction)
            : evaluateKnownByte(source, constants);
        continue;
      }
      if (destination === "de" && source) {
        continue;
      }
      if (destination === "[hl]" && source === "a" && hl) {
        memory.set(hl, requireA(instruction));
        continue;
      }
      const directWrite = destination?.match(/^\[([A-Za-z0-9_]+)\]$/);
      if (directWrite && source === "a") {
        memory.set(directWrite[1], requireA(instruction));
        continue;
      }
      fail();
    }
    if (instruction.opcode === "inc" && instruction.args[0] === "[hl]" && hl) {
      const current = memory.get(hl);
      if (current === undefined) fail();
      const value = (current! + 1) & 0xff;
      memory.set(hl, value);
      zero = value === 0;
      continue;
    }
    if (instruction.opcode === "inc" && instruction.args[0] === "a") {
      a = (requireA(instruction) + 1) & 0xff;
      zero = a === 0;
      continue;
    }
    if (instruction.opcode === "cp") {
      const value = requireA(instruction);
      const operand = evaluateKnownByte(instruction.args[0], constants);
      zero = value === operand;
      carry = value < operand;
      continue;
    }
    if (instruction.opcode === "and" && instruction.args.length === 1) {
      const value = requireA(instruction);
      a =
        instruction.args[0] === "a"
          ? value
          : value & evaluateKnownByte(instruction.args[0], constants);
      zero = a === 0;
      carry = false;
      continue;
    }
    if (instruction.opcode === "xor" && instruction.args[0] === "a") {
      a = 0;
      zero = true;
      carry = false;
      continue;
    }
    if (instruction.opcode === "srl" && instruction.args[0] === "a") {
      const value = requireA(instruction);
      carry = (value & 1) !== 0;
      a = value >>> 1;
      zero = a === 0;
      continue;
    }
    if (instruction.opcode === "sub" && instruction.args.length === 1) {
      const value = requireA(instruction);
      const operand = evaluateKnownByte(instruction.args[0], constants);
      a = (value - operand) & 0xff;
      zero = a === 0;
      carry = value < operand;
      continue;
    }
    if (instruction.opcode === "add" && instruction.args.length === 1) {
      const value = requireA(instruction);
      const operand = evaluateKnownByte(instruction.args[0], constants);
      const sum = value + operand;
      a = sum & 0xff;
      zero = a === 0;
      carry = sum > 0xff;
      continue;
    }
    if (instruction.opcode === "jr" || instruction.opcode === "jp") {
      const condition =
        instruction.args.length === 2 ? instruction.args[0] : null;
      const take =
        condition === null
          ? true
          : condition === "z"
            ? zero
            : condition === "nz"
              ? zero === null
                ? null
                : !zero
              : condition === "c"
                ? carry
                : condition === "nc"
                  ? carry === null
                    ? null
                    : !carry
                  : null;
      if (take === null) fail();
      if (take) {
        const target = instructionTarget(block, instruction, blocks);
        if (!target || !blocks.has(target)) fail();
        blockId = target!;
        instructionIndex = 0;
      }
      continue;
    }
    if (["call", "callfar", "farcall"].includes(instruction.opcode)) {
      const target = instructionTarget(block, instruction, blocks);
      const resolvedTarget = target ?? fail();
      if (resolvedTarget === "DeinitializeAllSprites") {
        certifyBulkSpriteDeinitializer(blocks);
        deinitialized = instruction.source_span;
        continue;
      }
      if (resolvedTarget === "PlaySpriteAnimations") {
        throw new Error(
          `persistent sprite handler ${entry} reaches nested PlaySpriteAnimations before the central scheduler`,
        );
      }
      if (resolvedTarget === "DelayFrames") {
        if (c === null || c <= 0) {
          throw new Error(
            `persistent sprite handler ${entry} calls DelayFrames with no exact positive C count`,
          );
        }
        preSchedulerWaits.push({
          target: "DelayFrames",
          frameCount: c,
          sourceSpan: instruction.source_span,
          implementationSourceSpan: certifyDelayFrames(blocks),
        });
        c = 0;
        a = null;
        zero = null;
        carry = null;
        continue;
      }
      const nextScene = certifyOuterIncrementHelper(resolvedTarget, blocks);
      if (nextScene?.symbol === "wJumptableIndex") {
        return {
          deinitialized,
          nextScene: true,
          nextSceneSourceSpan: instruction.source_span,
          preSchedulerWaits,
          hostCalls,
        };
      }
      certifyTrackedStateHost(resolvedTarget, trackedSymbols, blocks);
      hostCalls.push({
        target: resolvedTarget,
        sourceSpan: instruction.source_span,
      });
      a = null;
      zero = null;
      carry = null;
      continue;
    }
    if (instruction.opcode === "ret") {
      const shouldReturn =
        instruction.args.length === 0
          ? true
          : instruction.args[0] === "z"
            ? zero
            : instruction.args[0] === "nz"
              ? zero === null
                ? null
                : !zero
              : instruction.args[0] === "c"
                ? carry
                : instruction.args[0] === "nc"
                  ? carry === null
                    ? null
                    : !carry
                  : null;
      if (shouldReturn === null) fail();
      if (shouldReturn) {
        return {
          deinitialized,
          nextScene: false,
          nextSceneSourceSpan: null,
          preSchedulerWaits,
          hostCalls,
        };
      }
      continue;
    }
    fail();
  }
  throw new Error(`persistent sprite handler ${entry} exceeds step bound`);
};

const analyzePersistentSetupSpritePair = (
  entryIndex: number,
  entries: string[],
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): {
  instances: InitializedSpriteInstance[];
  programs: RuntimePresentationSpriteProgram[];
  nextAnimationIndex: number;
  handledEntryIndexes: number[];
} | null => {
  const setupEntry = entries[entryIndex];
  const firstHandlerEntry = entries[entryIndex + 1];
  if (
    !setupEntry ||
    !firstHandlerEntry ||
    !counterDrivenHandlerSymbol(firstHandlerEntry, blocks)
  ) {
    return null;
  }
  const handlerEntryIndexes: number[] = [];
  let clearBoundary: {
    entryIndex: number;
    sourceSpan: RuntimePresentationSourceSpan;
  } | null = null;
  let hasBulkTeardown = false;
  for (
    let candidateIndex = entryIndex + 1;
    candidateIndex < entries.length;
    candidateIndex += 1
  ) {
    const candidate = entries[candidateIndex];
    const exactClear = findExactSpriteClearCall(candidate, blocks);
    if (exactClear) {
      clearBoundary = { entryIndex: candidateIndex, sourceSpan: exactClear };
      break;
    }
    if (findPotentialSpriteInitializer(candidate, blocks, new Set())) {
      return null;
    }
    handlerEntryIndexes.push(candidateIndex);
    hasBulkTeardown = handlerFamily(candidate, blocks).some((block) =>
      block.instructions.some(
        (instruction) =>
          instructionTarget(block, instruction, blocks) ===
          "DeinitializeAllSprites",
      ),
    );
    if (hasBulkTeardown) break;
  }
  if (
    handlerEntryIndexes.length === 0 ||
    (!hasBulkTeardown && !clearBoundary)
  ) {
    return null;
  }
  const collected = collectInitializedSprites(
    setupEntry,
    blocks,
    constants,
    new Set(),
  );
  if (collected.instances.length === 0 || !collected.cleared) return null;
  const counterSymbol = counterDrivenHandlerSymbol(firstHandlerEntry, blocks)!;
  let nextAnimationIndex = 1;
  const instances = collected.instances.map((instance, structSlot) => {
    instance.structSlot = structSlot;
    instance.allocationDispatchTick = 1;
    instance.memory.index = nextAnimationIndex;
    nextAnimationIndex = (nextAnimationIndex + 1) & 0xff || 1;
    return instance;
  });
  const programs = instances.map((instance) =>
    compileInitializedSpriteProgram(instance, blocks, constants, loadSource),
  );
  if (programs.some((program) => program.callback.kind !== "direct")) {
    return null;
  }
  if (
    programs.some(
      (program) =>
        program.frameset.terminal.op === "delete" ||
        program.frameset_variants.some(
          (variant) => variant.terminal.op === "delete",
        ),
    )
  ) {
    throw new Error(
      `persistent sprite setup ${setupEntry} mixes frame-driven deletion with scene teardown`,
    );
  }
  const directPrograms = programs as Array<
    RuntimePresentationSpriteProgram & {
      callback: Extract<
        RuntimePresentationSpriteProgram["callback"],
        { kind: "direct" }
      >;
    }
  >;
  const tracked = new Set<string>([counterSymbol]);
  for (const program of directPrograms) {
    for (const read of program.callback.outer_memory_reads) {
      tracked.add(read.symbol);
    }
  }
  for (const handlerIndex of handlerEntryIndexes) {
    for (const instruction of handlerFamily(
      entries[handlerIndex],
      blocks,
    ).flatMap((block) => block.instructions)) {
      if (!["ld", "ldh"].includes(instruction.opcode)) continue;
      const read = instruction.args[1]?.match(/^\[([A-Za-z0-9_]+)\]$/);
      if (read && read[1] !== "hl") tracked.add(read[1]);
    }
  }
  const initializers = new Map<
    string,
    { value: number; sourceSpan: RuntimePresentationSourceSpan }
  >();
  for (const symbol of tracked) {
    const initialized = exactHandlerByteInitialization(
      setupEntry,
      symbol,
      blocks,
      constants,
    );
    if (!initialized) {
      throw new Error(
        `sprite callback outer memory ${symbol} is not initialized by ${setupEntry}`,
      );
    }
    initializers.set(symbol, initialized);
  }
  const memory = new Map(
    [...initializers].map(([symbol, initialized]) => [
      symbol,
      initialized.value,
    ]),
  );
  const domains = new Map(
    [...initializers].map(([symbol, initialized]) => [
      symbol,
      new Set([initialized.value]),
    ]),
  );
  const spriteRuntimeMemories = directPrograms.map((program) => ({
    ...program.initial_memory,
  }));
  const structDomains = directPrograms.map(
    (program, index) =>
      new Map(
        program.callback.struct_control_byte_domains.map(({ property }) => [
          property,
          new Set([spriteRuntimeMemories[index][property]]),
        ]),
      ),
  );
  const runScheduler = (schedulerTick: number): void => {
    for (let index = 0; index < directPrograms.length; index += 1) {
      const program = directPrograms[index];
      const runtimeMemory = spriteRuntimeMemories[index];
      executeDirectSpriteCallbackTick(
        program.callback,
        runtimeMemory,
        memory,
        schedulerTick,
        blocks,
        constants,
      );
      for (const [property, values] of structDomains[index]) {
        values.add(runtimeMemory[property]);
      }
    }
  };
  let schedulerTicks = 1;
  runScheduler(schedulerTicks);
  const deinitializers: Array<{
    dispatcherEntry: number;
    sourceSpan: RuntimePresentationSourceSpan;
    dispatchTick: number;
  }> = [];
  const outerSceneAdvances: Array<{
    dispatcherEntry: number;
    sourceSpan: RuntimePresentationSourceSpan;
    dispatchTick: number;
  }> = [];
  const preSchedulerWaits: RuntimePresentationSpriteProgram["lifetime"]["pre_scheduler_waits"] =
    [];
  let spriteActive = true;
  const handlerHostOperations = new Map<
    string,
    {
      target: string;
      dispatcher_entry: number;
      source_span: RuntimePresentationSourceSpan;
      dispatch_ticks: number[];
    }
  >();
  const simulatedHandlerIndexes: number[] = [];
  for (const handlerIndex of handlerEntryIndexes) {
    const handlerEntry = entries[handlerIndex];
    simulatedHandlerIndexes.push(handlerIndex);
    let reachedNextScene = false;
    for (let dispatchTick = 1; dispatchTick <= 256; dispatchTick += 1) {
      const tick = runPersistentHandlerTick(
        handlerEntry,
        memory,
        tracked,
        blocks,
        constants,
      );
      for (const call of tick.hostCalls) {
        const key =
          `${handlerIndex}:${call.target}:` +
          `${call.sourceSpan.file}:${call.sourceSpan.start_line}`;
        const operation = handlerHostOperations.get(key) ?? {
          target: call.target,
          dispatcher_entry: handlerIndex,
          source_span: call.sourceSpan,
          dispatch_ticks: [],
        };
        operation.dispatch_ticks.push(dispatchTick);
        handlerHostOperations.set(key, operation);
      }
      for (const wait of tick.preSchedulerWaits) {
        preSchedulerWaits.push({
          target: wait.target,
          dispatcher_entry: handlerIndex,
          dispatch_tick: dispatchTick,
          frame_count: wait.frameCount,
          before_scheduler_step: true,
          source_span: wait.sourceSpan,
          implementation_source_span: wait.implementationSourceSpan,
        });
      }
      for (const symbol of tracked) {
        const value = memory.get(symbol);
        if (value === undefined) {
          throw new Error(
            `tracked persistent sprite byte ${symbol} is missing`,
          );
        }
        domains.get(symbol)!.add(value!);
      }
      if (tick.deinitialized) {
        deinitializers.push({
          dispatcherEntry: handlerIndex,
          sourceSpan: tick.deinitialized,
          dispatchTick,
        });
        spriteActive = false;
      }
      if (tick.nextScene) {
        if (!tick.nextSceneSourceSpan) {
          throw new Error(`${handlerEntry} next-scene source span is missing`);
        }
        outerSceneAdvances.push({
          dispatcherEntry: handlerIndex,
          sourceSpan: tick.nextSceneSourceSpan,
          dispatchTick,
        });
        reachedNextScene = true;
      }
      if (spriteActive) {
        schedulerTicks += 1;
        runScheduler(schedulerTicks);
      }
      if (tick.nextScene) break;
    }
    if (!reachedNextScene) {
      throw new Error(
        `persistent sprite in ${handlerEntry} has no finite scene advance`,
      );
    }
    if (!spriteActive) break;
  }
  const firstBulkDeinitializer = deinitializers[0] ?? null;
  if (!firstBulkDeinitializer && !clearBoundary) {
    throw new Error(
      `persistent sprite after ${setupEntry} has no exact source teardown`,
    );
  }
  const deinitializer = firstBulkDeinitializer
    ? {
        op: "deinitialize_all_sprites" as const,
        dispatcher_entry: firstBulkDeinitializer.dispatcherEntry,
        dispatch_tick: firstBulkDeinitializer.dispatchTick,
        before_scheduler_step: true as const,
        source_span: firstBulkDeinitializer.sourceSpan,
        implementation_source_span: certifyBulkSpriteDeinitializer(blocks),
        reachable_dispatch_ticks: deinitializers
          .filter(
            (candidate) =>
              candidate.dispatcherEntry ===
              firstBulkDeinitializer.dispatcherEntry,
          )
          .map((candidate) => candidate.dispatchTick),
      }
    : {
        op: "clear_sprite_anims" as const,
        dispatcher_entry: clearBoundary!.entryIndex,
        dispatch_tick: 1,
        before_scheduler_step: true as const,
        source_span: clearBoundary!.sourceSpan,
        implementation_source_span: certifyClearSpriteAnimations(blocks),
        reachable_dispatch_ticks: [1],
      };
  const outerByteDomains = [...domains].map(([symbol, values]) => {
    const sorted = sortedDomain(values);
    return {
      symbol,
      initialized_value: initializers.get(symbol)!.value,
      minimum: sorted[0],
      maximum: sorted.at(-1)!,
      values: sorted,
      initializer_source_span: initializers.get(symbol)!.sourceSpan,
    };
  });
  for (let index = 0; index < directPrograms.length; index += 1) {
    const program = directPrograms[index];
    program.callback.struct_control_byte_domains = [
      ...structDomains[index],
    ].map(([property, values]) => {
      const sorted = sortedDomain(values);
      return {
        property,
        initialized_value: program.initial_memory[property],
        minimum: sorted[0],
        maximum: sorted.at(-1)!,
        values: sorted,
      };
    });
    program.lifetime = {
      allocation_dispatcher_entry: entryIndex,
      allocation_dispatch_tick: 1,
      active_dispatcher_entries: [
        entryIndex,
        ...simulatedHandlerIndexes,
        ...(clearBoundary ? [clearBoundary.entryIndex] : []),
      ],
      scheduler_ticks: schedulerTicks,
      deinitialized_after_dispatch_tick: deinitializer.dispatch_tick,
      callback_before_frame_update: true,
      outer_byte_domains: outerByteDomains,
      deinitializer,
      outer_scene_advances: outerSceneAdvances.map((advance) => ({
        dispatcher_entry: advance.dispatcherEntry,
        dispatch_tick: advance.dispatchTick,
        source_span: advance.sourceSpan,
      })),
      pre_scheduler_waits: preSchedulerWaits,
      handler_host_operations: [...handlerHostOperations.values()],
    };
  }
  return {
    instances,
    programs,
    nextAnimationIndex:
      deinitializer.op === "clear_sprite_anims" ? 1 : nextAnimationIndex,
    handledEntryIndexes: [
      ...simulatedHandlerIndexes,
      ...(clearBoundary ? [clearBoundary.entryIndex] : []),
    ],
  };
};

type CarriedSpriteAllocationState = {
  nextAnimationIndex: number;
  allStructSlotsFree: boolean;
};

const findExactSpriteClearCall = (
  entry: string,
  blocks: Map<string, ParsedAsmBlock>,
): RuntimePresentationSourceSpan | null => {
  const family = handlerFamily(entry, blocks);
  const matches = family.flatMap((block) =>
    block.instructions.flatMap((instruction, instructionIndex) =>
      ["call", "callfar", "farcall"].includes(instruction.opcode) &&
      instructionTarget(block, instruction, blocks) === "ClearSpriteAnims"
        ? [{ block, instruction, instructionIndex }]
        : [],
    ),
  );
  if (matches.length > 1) {
    throw new Error(`${entry} has duplicate ClearSpriteAnims teardown calls`);
  }
  const match = matches[0];
  if (!match) return null;
  if (
    match.block.id !== entry ||
    match.block.instructions
      .slice(0, match.instructionIndex)
      .some((instruction) => ["jr", "jp", "ret"].includes(instruction.opcode))
  ) {
    throw new Error(
      `${entry} ClearSpriteAnims teardown is not unconditionally reached before the scheduler`,
    );
  }
  return match.instruction.source_span;
};

const analyzeCounterDrivenDirectSpritePrograms = (
  entryIndex: number,
  entries: string[],
  counterSymbol: string,
  inheritedState: CarriedSpriteAllocationState,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): {
  instances: InitializedSpriteInstance[];
  programs: RuntimePresentationSpriteProgram[];
  nextState: CarriedSpriteAllocationState;
} => {
  const entry = entries[entryIndex];
  const nextEntry = entries[entryIndex + 1];
  if (!entry || !nextEntry || !inheritedState.allStructSlotsFree) {
    throw new Error(
      `counter-driven direct sprite handler ${String(entry)} lacks an exact free-slot predecessor state`,
    );
  }
  const counterInitializer = exactHandlerByteInitialization(
    entries[entryIndex - 1],
    counterSymbol,
    blocks,
    constants,
  );
  if (!counterInitializer || counterInitializer.value !== 0) {
    throw new Error(
      `counter-driven direct sprite handler ${entry} has no exact zero counter initialization`,
    );
  }
  let counter = 0;
  let sceneEndTick: number | null = null;
  const allocations: CounterDrivenAllocationCall[] = [];
  const counterDomain = new Set([0]);
  const handlerHostOperations = new Map<
    string,
    {
      target: string;
      dispatcher_entry: number;
      source_span: RuntimePresentationSourceSpan;
      dispatch_ticks: number[];
    }
  >();
  for (let dispatchTick = 1; dispatchTick <= 256; dispatchTick += 1) {
    const tick = runCounterDrivenHandlerTick(
      entry,
      counterSymbol,
      counter,
      dispatchTick,
      blocks,
      constants,
    );
    counter = tick.counter;
    counterDomain.add(counter);
    allocations.push(...tick.allocations);
    for (const call of tick.hostCalls) {
      certifyTrackedStateHost(call.target, new Set([counterSymbol]), blocks);
      const key = `${call.target}:${call.sourceSpan.file}:${call.sourceSpan.start_line}`;
      const operation = handlerHostOperations.get(key) ?? {
        target: call.target,
        dispatcher_entry: entryIndex,
        source_span: call.sourceSpan,
        dispatch_ticks: [],
      };
      operation.dispatch_ticks.push(dispatchTick);
      handlerHostOperations.set(key, operation);
    }
    if (tick.nextScene) {
      sceneEndTick = dispatchTick;
      break;
    }
  }
  if (sceneEndTick === null || allocations.length === 0) {
    throw new Error(
      `counter-driven direct sprite handler ${entry} has no finite allocation lifetime`,
    );
  }
  const clearCall = findExactSpriteClearCall(nextEntry, blocks);
  if (!clearCall) {
    throw new Error(
      `persistent ${entry} sprites have no exact next-scene ClearSpriteAnims teardown`,
    );
  }
  const clearImplementation = certifyClearSpriteAnimations(blocks);
  let nextAnimationIndex = inheritedState.nextAnimationIndex;
  const instances = allocations.map((allocation, structSlot) => {
    const instance = compileDirectSpriteAllocation(
      allocation,
      nextAnimationIndex,
      structSlot,
      constants,
    );
    nextAnimationIndex = (nextAnimationIndex + 1) & 0xff || 1;
    return instance;
  });
  const programs = instances.map((instance) => {
    const program = compileInitializedSpriteProgram(
      instance,
      blocks,
      constants,
      loadSource,
    );
    if (program.callback.kind !== "direct") {
      throw new Error(
        `counter-driven direct sprite ${program.instance} has a non-direct callback`,
      );
    }
    if (program.frameset.terminal.op === "delete") {
      throw new Error(
        `counter-driven direct sprite ${program.instance} is not persistent through its next-scene clear`,
      );
    }
    const runtimeMemory = { ...program.initial_memory };
    const controlDomains = new Map(
      program.callback.struct_control_byte_domains.map(({ property }) => [
        property,
        new Set([runtimeMemory[property]]),
      ]),
    );
    for (
      let dispatchTick = instance.allocationDispatchTick;
      dispatchTick <= sceneEndTick!;
      dispatchTick += 1
    ) {
      executeDirectSpriteCallbackTick(
        program.callback,
        runtimeMemory,
        new Map(),
        dispatchTick,
        blocks,
        constants,
      );
      for (const [property, values] of controlDomains) {
        values.add(runtimeMemory[property]);
      }
    }
    program.callback.struct_control_byte_domains = [...controlDomains].map(
      ([property, values]) => {
        const sorted = sortedDomain(values);
        return {
          property,
          initialized_value: program.initial_memory[property],
          minimum: sorted[0],
          maximum: sorted.at(-1)!,
          values: sorted,
        };
      },
    );
    const sortedCounterDomain = sortedDomain(counterDomain);
    program.lifetime = {
      allocation_dispatcher_entry: entryIndex,
      allocation_dispatch_tick: instance.allocationDispatchTick,
      active_dispatcher_entries: [entryIndex, entryIndex + 1],
      scheduler_ticks: sceneEndTick! - instance.allocationDispatchTick + 1,
      deinitialized_after_dispatch_tick: 1,
      callback_before_frame_update: true,
      outer_byte_domains: [
        {
          symbol: counterSymbol,
          initialized_value: counterInitializer.value,
          minimum: sortedCounterDomain[0],
          maximum: sortedCounterDomain.at(-1)!,
          values: sortedCounterDomain,
          initializer_source_span: counterInitializer.sourceSpan,
        },
      ],
      deinitializer: {
        op: "clear_sprite_anims",
        dispatcher_entry: entryIndex + 1,
        dispatch_tick: 1,
        before_scheduler_step: true,
        source_span: clearCall,
        implementation_source_span: clearImplementation,
        reachable_dispatch_ticks: [1],
      },
      outer_scene_advances: [],
      pre_scheduler_waits: [],
      handler_host_operations: [...handlerHostOperations.values()]
        .map((operation) => ({
          ...operation,
          dispatch_ticks: operation.dispatch_ticks.filter(
            (tick) => tick >= instance.allocationDispatchTick,
          ),
        }))
        .filter((operation) => operation.dispatch_ticks.length > 0),
    };
    return program;
  });
  return {
    instances,
    programs,
    nextState: { nextAnimationIndex: 1, allStructSlotsFree: true },
  };
};

type DynamicSpritePrograms = {
  instances: InitializedSpriteInstance[];
  programs: RuntimePresentationSpriteProgram[];
  deferred: DeferredSpriteInitializer | null;
};

const analyzeCounterDrivenSpritePrograms = (
  dispatcher: ParsedAsmBlock,
  entries: string[],
  domain: ReadonlySet<number>,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): DynamicSpritePrograms | null => {
  if (!findSpriteSchedulerLoopSite(dispatcher, blocks)) return null;
  validateSpriteRuntimeContracts(blocks, loadSource);
  const instances: InitializedSpriteInstance[] = [];
  const programs: RuntimePresentationSpriteProgram[] = [];
  const handledEntries = new Set<number>();
  let carriedAllocationState: CarriedSpriteAllocationState | null = null;
  for (const entryIndex of sortedDomain(domain)) {
    if (handledEntries.has(entryIndex)) continue;
    const entry = entries[entryIndex];
    if (!entry) continue;
    const deferred = findPotentialSpriteInitializer(entry, blocks, new Set());
    if (!deferred) continue;
    const counterSymbol = counterDrivenHandlerSymbol(entry, blocks);
    if (!counterSymbol) {
      const persistent = analyzePersistentSetupSpritePair(
        entryIndex,
        entries,
        blocks,
        constants,
        loadSource,
      );
      if (persistent) {
        instances.push(...persistent.instances);
        programs.push(...persistent.programs);
        carriedAllocationState = {
          nextAnimationIndex: persistent.nextAnimationIndex,
          allStructSlotsFree: true,
        };
        for (const handledEntry of persistent.handledEntryIndexes) {
          handledEntries.add(handledEntry);
        }
        continue;
      }
      return {
        instances,
        programs,
        deferred: { entryIndex, entry, ...deferred },
      };
    }
    if (
      !previousHandlerInitializesCounter(
        entries,
        entryIndex,
        counterSymbol,
        blocks,
      )
    ) {
      return {
        instances,
        programs,
        deferred: { entryIndex, entry, ...deferred },
      };
    }
    if (!previousHandlerClearsSpriteInstances(entries, entryIndex, blocks)) {
      if (carriedAllocationState) {
        const direct = analyzeCounterDrivenDirectSpritePrograms(
          entryIndex,
          entries,
          counterSymbol,
          carriedAllocationState,
          blocks,
          constants,
          loadSource,
        );
        instances.push(...direct.instances);
        programs.push(...direct.programs);
        carriedAllocationState = direct.nextState;
        continue;
      }
      let counter = 0;
      for (let dispatchTick = 1; dispatchTick <= 256; dispatchTick += 1) {
        const tick = runCounterDrivenHandlerTick(
          entry,
          counterSymbol,
          counter,
          dispatchTick,
          blocks,
          constants,
        );
        counter = tick.counter;
        const first = tick.allocations[0];
        if (first) {
          if (first.target !== "InitSpriteAnimStruct") {
            throw new Error(
              `counter-driven sprite handler ${entry} has no exact prior ClearSpriteAnims reset`,
            );
          }
          return {
            instances,
            programs,
            deferred: {
              entryIndex,
              entry,
              callSite: first.sourceSpan,
              initializer: first.sourceSpan,
            },
          };
        }
        if (tick.nextScene) break;
      }
      throw new Error(
        `counter-driven sprite handler ${entry} has no exact prior ClearSpriteAnims reset`,
      );
    }
    let counter = 0;
    let nextAnimationIndex = 1;
    const handlerInstances: InitializedSpriteInstance[] = [];
    let sceneEndTick: number | null = null;
    for (let dispatchTick = 1; dispatchTick <= 256; dispatchTick += 1) {
      const tick = runCounterDrivenHandlerTick(
        entry,
        counterSymbol,
        counter,
        dispatchTick,
        blocks,
        constants,
      );
      counter = tick.counter;
      for (const allocation of tick.allocations) {
        const compiled = compileSpriteAllocationHelper(
          allocation,
          nextAnimationIndex,
          blocks,
          constants,
        );
        nextAnimationIndex = compiled.nextAnimationIndex;
        handlerInstances.push(...compiled.instances);
      }
      if (tick.nextScene) {
        sceneEndTick = dispatchTick;
        break;
      }
    }
    if (sceneEndTick === null) {
      throw new Error(
        `counter-driven sprite handler ${entry} does not reach its next scene`,
      );
    }
    const handlerPrograms = handlerInstances.map((instance) =>
      compileInitializedSpriteProgram(instance, blocks, constants, loadSource),
    );
    const waves = new Map<number, RuntimePresentationSpriteProgram[]>();
    for (const program of handlerPrograms) {
      const wave = waves.get(program.lifetime.allocation_dispatch_tick) ?? [];
      wave.push(program);
      waves.set(program.lifetime.allocation_dispatch_tick, wave);
    }
    const waveTicks = [...waves.keys()].sort((left, right) => left - right);
    for (let index = 0; index < waveTicks.length; index += 1) {
      const allocationTick = waveTicks[index];
      const nextBoundary = waveTicks[index + 1] ?? sceneEndTick + 1;
      for (const program of waves.get(allocationTick)!) {
        const deinitialized =
          program.lifetime.deinitialized_after_dispatch_tick;
        if (deinitialized === null || deinitialized >= nextBoundary) {
          throw new Error(
            `dynamic sprite ${program.instance} does not deinitialize within its exact delete lifetime`,
          );
        }
      }
    }
    instances.push(...handlerInstances);
    programs.push(...handlerPrograms);
    carriedAllocationState = null;
  }
  return { instances, programs, deferred: null };
};

const analyzeSpriteSchedulerBridge = (
  dispatcher: ParsedAsmBlock,
  entries: string[],
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  loadSource: (relativePath: string) => LoadedSource,
): SpriteSchedulerBridge | null => {
  const loopSite = findSpriteSchedulerLoopSite(dispatcher, blocks);
  if (!loopSite) return null;
  const { callSite, scheduler } = loopSite;

  const precedingCalls = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === callSite.block.file &&
        candidate.globalLabel === callSite.block.globalLabel &&
        candidate.startLine <= callSite.block.startLine,
    )
    .sort((left, right) => left.startLine - right.startLine)
    .flatMap((candidate) =>
      candidate.instructions
        .filter(
          (instruction) =>
            instruction.source_span.start_line <
              scheduler.source_span.start_line &&
            ["call", "callfar", "farcall"].includes(instruction.opcode),
        )
        .map((instruction) => ({ candidate, instruction })),
    );
  const initialized: InitializedSpriteInstance[] = [];
  let cleared = false;
  for (const preceding of precedingCalls) {
    const target = instructionTarget(
      preceding.candidate,
      preceding.instruction,
      blocks,
    );
    if (!target || !blocks.has(target)) continue;
    const result = collectInitializedSprites(
      target,
      blocks,
      constants,
      new Set(),
    );
    initialized.push(...result.instances);
    cleared ||= result.cleared;
  }
  const uniqueInitialized = [
    ...new Map(
      initialized.map((instance) => [instance.instance, instance]),
    ).values(),
  ];
  if (uniqueInitialized.length === 0) return null;
  if (!cleared) {
    throw new Error(
      "sprite initializer closure has no exact ClearSpriteAnims state reset",
    );
  }
  validateSpriteRuntimeContracts(blocks, loadSource);
  const programs = uniqueInitialized.map((instance) =>
    compileInitializedSpriteProgram(instance, blocks, constants, loadSource),
  );

  const beforeCall = callSite.block.instructions.slice(0, callSite.index);
  const stateLoad = [...beforeCall]
    .reverse()
    .find(
      (instruction) =>
        instruction.opcode === "ld" &&
        instruction.args[0] === "a" &&
        /^\[w[A-Za-z0-9_]+\]$/.test(instruction.args[1] ?? ""),
    );
  if (!stateLoad) {
    throw new Error("sprite scheduler bridge has no exact outer state load");
  }
  const outerSymbol = stateLoad.args[1].slice(1, -1);
  const relevantEffects = programs.flatMap((program) =>
    program.outer_state_effects.filter(
      (effect) => effect.symbol === outerSymbol,
    ),
  );
  if (relevantEffects.length !== 1) {
    throw new Error(
      `sprite callback has no exact single outer-state write to ${outerSymbol}`,
    );
  }
  const passiveEntries = entries.flatMap((entry, index) => {
    const actions = stateMachineActions(entry, outerSymbol, blocks, constants);
    return actions.length === 1 && actions[0].kind === "keep" ? [index] : [];
  });
  if (passiveEntries.length !== 1) {
    throw new Error(
      `sprite outer-state bridge cannot identify one passive dispatcher state; found ${passiveEntries.join(",")}`,
    );
  }
  const operations: RuntimePresentationSpriteOperation[] = [
    ...uniqueInitialized.map((instance) => ({
      op: "sprite_init" as const,
      instance: instance.instance,
      object: {
        symbol: instance.objectSymbol,
        value: instance.objectValue,
      },
      source_span: instance.sourceSpan,
      allocation_source_span: instance.allocationSourceSpan,
    })),
    {
      op: "sprite_scheduler_step",
      instances: uniqueInitialized.map((instance) => instance.instance),
      source_span: scheduler.source_span,
      before_host_call: "DelayFrame",
    },
  ];
  return {
    actions: new Map([[passiveEntries[0], [{ kind: "increment" }]]]),
    operations,
    programs,
  };
};

const guardedTableDomain = (
  block: ParsedAsmBlock,
  tableLength: number,
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
): ByteDomain | null => {
  const family = [...blocks.values()]
    .filter(
      (candidate) =>
        candidate.file === block.file &&
        candidate.globalLabel === block.globalLabel &&
        candidate.startLine <= block.startLine,
    )
    .sort((left, right) => left.startLine - right.startLine);
  const instructions = family.flatMap((candidate) => candidate.instructions);
  const compareIndex = instructions.findIndex(
    (instruction) =>
      instruction.opcode === "cp" && instruction.args.length === 1,
  );
  if (compareIndex < 0) return null;
  const compare = instructions[compareIndex];
  const guardIndex = compareIndex + 1;
  const guard = instructions[guardIndex];
  if (
    !guard ||
    !["jr", "jp"].includes(guard.opcode) ||
    guard.args.length !== 2 ||
    guard.args[0] !== "c"
  ) {
    return null;
  }
  const guardedTarget = resolveControlTarget(
    block,
    guard.args[1],
    blocks,
    guard.source_span.start_line,
  );
  if (guardedTarget !== block.id) return null;
  const fallback = instructions
    .slice(guardIndex + 1)
    .filter(
      (instruction) => instruction.source_span.start_line < block.startLine,
    );
  let comparedValue: number;
  try {
    comparedValue = evaluateAsmInteger(compare.args[0], constants);
  } catch {
    return null;
  }
  if (
    comparedValue !== tableLength ||
    fallback.length !== 1 ||
    fallback[0].opcode !== "xor" ||
    fallback[0].args.length !== 1 ||
    fallback[0].args[0] !== "a"
  ) {
    return null;
  }
  return new Set(Array.from({ length: tableLength }, (_, value) => value));
};

const serializeByteDomain = (
  domain: ReadonlySet<number> | null,
): { minimum: number; maximum: number; values: number[] } | null => {
  if (!domain || domain.size === 0) return null;
  const values = sortedDomain(domain);
  return {
    minimum: values[0],
    maximum: values.at(-1)!,
    values,
  };
};

const inferIndirectTableDomain = (
  block: ParsedAsmBlock,
  tableName: string,
  entries: string[],
  blocks: Map<string, ParsedAsmBlock>,
  constants: ReadonlyMap<string, number>,
  spriteContext: SpriteDomainInferenceContext,
): RuntimePresentationControlFlow["indirect_tables"][number]["index_domain"] => {
  const directDomain =
    guardedTableDomain(block, entries.length, blocks, constants) ??
    inferTypedMenuSelectionDomain(block, blocks, constants) ??
    inferStateMachineDomain(block, entries, blocks, constants);
  if (directDomain) {
    try {
      const dynamic = analyzeCounterDrivenSpritePrograms(
        block,
        entries,
        directDomain,
        blocks,
        constants,
        spriteContext.loadSource,
      );
      if (dynamic) {
        for (const program of dynamic.programs) {
          if (
            !spriteContext.programs.some(
              (candidate) => candidate.instance === program.instance,
            )
          ) {
            spriteContext.programs.push(program);
          }
        }
        for (const instance of dynamic.instances) {
          spriteContext.operations.push({
            op: "sprite_init",
            instance: instance.instance,
            object: {
              symbol: instance.objectSymbol,
              value: instance.objectValue,
            },
            source_span: instance.sourceSpan,
            allocation_source_span: instance.allocationSourceSpan,
          });
        }
        const loopSite = findSpriteSchedulerLoopSite(block, blocks);
        if (dynamic.instances.length > 0 && loopSite) {
          spriteContext.operations.push({
            op: "sprite_scheduler_step",
            instances: dynamic.instances.map((instance) => instance.instance),
            source_span: loopSite.scheduler.source_span,
            before_host_call: "DelayFrame",
          });
        }
      }
      if (dynamic?.deferred) {
        const deferred = dynamic.deferred;
        spriteContext.diagnostics.push({
          table: tableName,
          message:
            `reachable dispatcher entry ${deferred.entryIndex} (${deferred.entry}) ` +
            `calls a dynamically initialized sprite at ` +
            `${deferred.initializer.file}:${deferred.initializer.start_line}; ` +
            "its cross-handler outer-state and lifetime path is not yet source-certified",
          source_span: deferred.callSite,
        });
      }
    } catch (error) {
      spriteContext.diagnostics.push({
        table: tableName,
        message: error instanceof Error ? error.message : String(error),
        source_span: parsedBlockSourceSpan(block),
      });
    }
    return serializeByteDomain(directDomain);
  }
  let bridge: SpriteSchedulerBridge | null;
  try {
    bridge = analyzeSpriteSchedulerBridge(
      block,
      entries,
      blocks,
      constants,
      spriteContext.loadSource,
    );
  } catch (error) {
    spriteContext.diagnostics.push({
      table: tableName,
      message: error instanceof Error ? error.message : String(error),
      source_span: parsedBlockSourceSpan(block),
    });
    return null;
  }
  if (!bridge) return null;
  const bridgedDomain = inferStateMachineDomain(
    block,
    entries,
    blocks,
    constants,
    bridge.actions,
  );
  if (!bridgedDomain) {
    spriteContext.diagnostics.push({
      table: tableName,
      message:
        "sprite callback outer-state bridge does not close the dispatcher domain",
      source_span: parsedBlockSourceSpan(block),
    });
    return null;
  }
  for (const operation of bridge.operations) {
    const key = `${operation.op}:${operation.source_span.file}:${operation.source_span.start_line}`;
    if (
      !spriteContext.operations.some(
        (candidate) =>
          `${candidate.op}:${candidate.source_span.file}:${candidate.source_span.start_line}` ===
          key,
      )
    ) {
      spriteContext.operations.push(operation);
    }
  }
  for (const program of bridge.programs) {
    if (
      !spriteContext.programs.some(
        (candidate) => candidate.instance === program.instance,
      )
    ) {
      spriteContext.programs.push(program);
    }
  }
  return serializeByteDomain(bridgedDomain);
};

export function analyzeRuntimePresentationControlFlow(
  options: Pick<
    BuildRuntimeTitlePresentationProgramOptions,
    "disassemblyRoot" | "readSource"
  >,
): RuntimePresentationControlFlow {
  const loadControlFlowSource = (file: string): LoadedSource =>
    loadSource(file, {
      ...options,
      audioAssetIds: new Set(),
      runtimeSpawnIdentifiers: new Set(),
    });
  const sources = PROGRAM_SOURCE_FILES.map(loadControlFlowSource);
  const spriteSources = SPRITE_SOURCE_FILES.map(loadControlFlowSource);
  const constantSources = CONSTANT_SOURCE_FILES.map(loadControlFlowSource);
  const parsed = parseAsmBlocks([...sources, ...spriteSources]);
  const constants = parseAsmConstants([
    ...sources,
    ...spriteSources,
    ...constantSources,
  ]);
  const pending = Object.values(PRESENTATION_ENTRYPOINT_LABELS);
  const reachable = new Set<string>();
  const externalCalls: RuntimePresentationControlFlow["external_calls"] = [];
  const indirectTables: RuntimePresentationControlFlow["indirect_tables"] = [];
  const spriteContext: SpriteDomainInferenceContext = {
    operations: [],
    programs: [],
    diagnostics: [],
    loadSource: loadControlFlowSource,
  };

  while (pending.length > 0) {
    const blockId = pending.shift()!;
    if (reachable.has(blockId)) continue;
    const block = parsed.get(blockId);
    if (!block) {
      throw new Error(
        `Runtime presentation control target ${blockId} is missing`,
      );
    }
    reachable.add(blockId);
    for (let index = 0; index < block.instructions.length; index += 1) {
      const instruction = block.instructions[index];
      const target = instructionTarget(block, instruction, parsed);
      if (target) {
        if (["hl", "de", "bc"].includes(target)) {
          const table = precedingTableSymbol(block, index);
          if (!table) {
            throw new Error(
              `Unresolved indirect runtime presentation target at ${block.file}:${instruction.source_span.start_line}`,
            );
          }
          const entries = tableEntries(parsed, table);
          if (entries.length === 0) {
            throw new Error(
              `Indirect runtime presentation table ${table} has no exact entries`,
            );
          }
          indirectTables.push({
            source_span: instruction.source_span,
            table,
            entries,
            index_domain: inferIndirectTableDomain(
              block,
              table,
              entries,
              parsed,
              constants,
              spriteContext,
            ),
          });
          pending.push(...entries.filter((entry) => parsed.has(entry)));
        } else if (TYPED_SPRITE_BOUNDARIES.has(target)) {
          externalCalls.push({
            target,
            call_form: ["call", "callfar", "farcall"].includes(
              instruction.opcode,
            )
              ? (instruction.opcode as "call" | "callfar" | "farcall")
              : "jump",
            args: [],
            source_span: instruction.source_span,
          });
        } else if (parsed.has(target)) {
          pending.push(target);
        } else {
          externalCalls.push({
            target,
            call_form: ["call", "callfar", "farcall"].includes(
              instruction.opcode,
            )
              ? (instruction.opcode as "call" | "callfar" | "farcall")
              : target === "Reset" || target === "Init"
                ? "restart"
                : "jump",
            args: [],
            source_span: instruction.source_span,
          });
        }
      }
      if (instruction.opcode === "jumptable" && instruction.args.length >= 1) {
        const table = resolveControlTarget(
          block,
          instruction.args[0],
          parsed,
          instruction.source_span.start_line,
        );
        const entries = tableEntries(parsed, table);
        if (entries.length === 0) {
          throw new Error(
            `Indirect runtime presentation table ${table} has no exact entries`,
          );
        }
        indirectTables.push({
          source_span: instruction.source_span,
          table,
          entries,
          index_domain: inferIndirectTableDomain(
            block,
            table,
            entries,
            parsed,
            constants,
            spriteContext,
          ),
        });
        pending.push(...entries.filter((entry) => parsed.has(entry)));
      }
      if (instruction.opcode === "rst" && instruction.args[0] === "JumpTable") {
        const table = precedingTableSymbol(block, index);
        if (!table) {
          throw new Error(
            `JumpTable at ${block.file}:${instruction.source_span.start_line} has no source table`,
          );
        }
        const entries = tableEntries(parsed, table);
        if (entries.length === 0) {
          throw new Error(
            `Indirect runtime presentation table ${table} has no exact entries`,
          );
        }
        indirectTables.push({
          source_span: instruction.source_span,
          table,
          entries,
          index_domain: inferIndirectTableDomain(
            block,
            table,
            entries,
            parsed,
            constants,
            spriteContext,
          ),
        });
        pending.push(...entries.filter((entry) => parsed.has(entry)));
      }
    }
    const last = block.instructions.at(-1);
    if (
      block.nextBlock &&
      (!isUnconditionalTerminal(last) || isConditionalControl(last!))
    ) {
      pending.push(block.nextBlock);
    }
  }

  const outputBlocks: RuntimePresentationControlFlow["blocks"] = {};
  for (const blockId of [...reachable].sort()) {
    const block = parsed.get(blockId)!;
    const directTargets = block.instructions
      .map((instruction) => instructionTarget(block, instruction, parsed))
      .filter(
        (target): target is string =>
          !!target &&
          parsed.has(target) &&
          !["hl", "de", "bc"].includes(target),
      );
    const endLine =
      block.instructions.at(-1)?.source_span.end_line ?? block.startLine;
    outputBlocks[blockId] = {
      source_span: {
        file: block.file,
        start_line: block.startLine,
        end_line: endLine,
      },
      instructions: block.instructions,
      direct_targets: [...new Set(directTargets)],
      fallthrough:
        block.nextBlock && !isUnconditionalTerminal(block.instructions.at(-1))
          ? block.nextBlock
          : null,
    };
  }
  return {
    entrypoints: { ...PRESENTATION_ENTRYPOINT_LABELS },
    blocks: outputBlocks,
    indirect_tables: indirectTables,
    external_calls: externalCalls,
    sprite_operations: spriteContext.operations,
    sprite_programs: spriteContext.programs,
    sprite_diagnostics: spriteContext.diagnostics,
  };
}

export function requireClosedRuntimePresentationControlFlow(
  options: Pick<
    BuildRuntimeTitlePresentationProgramOptions,
    "disassemblyRoot" | "readSource"
  >,
): RuntimePresentationControlFlow {
  const controlFlow = analyzeRuntimePresentationControlFlow(options);
  for (const table of controlFlow.indirect_tables) {
    if (!table.index_domain) {
      throw new Error(
        `Runtime presentation indirect table ${table.table} at ${table.source_span.file}:${table.source_span.start_line} has no exact index-domain proof`,
      );
    }
    for (const value of table.index_domain.values) {
      const target = table.entries[value];
      if (!target) {
        throw new Error(
          `Runtime presentation indirect table ${table.table} at ${table.source_span.file}:${table.source_span.start_line} has reachable index ${value} outside its ${table.entries.length} exact entries`,
        );
      }
      if (!controlFlow.blocks[target]) {
        throw new Error(
          `Runtime presentation indirect table ${table.table} at ${table.source_span.file}:${table.source_span.start_line} has reachable target ${target} at index ${value} with no executable source block`,
        );
      }
    }
  }
  const spriteDiagnostic = controlFlow.sprite_diagnostics[0];
  if (spriteDiagnostic) {
    throw new Error(
      `Runtime presentation sprite closure ${spriteDiagnostic.table} at ` +
        `${spriteDiagnostic.source_span.file}:${spriteDiagnostic.source_span.start_line} ` +
        `is not executable: ${spriteDiagnostic.message}`,
    );
  }
  return controlFlow;
}

const runtimePresentationInstructionTarget = (
  instruction: RuntimePresentationAsmInstruction,
): string | null =>
  ["call", "callfar", "farcall", "jp", "jr"].includes(instruction.opcode)
    ? (instruction.args.at(-1) ?? null)
    : null;

const instructionSignature = (
  instruction: RuntimePresentationAsmInstruction,
): string =>
  instruction.args.length > 0
    ? `${instruction.opcode} ${instruction.args.join(", ")}`
    : instruction.opcode;

const exactRoutineSpan = (
  block: ParsedAsmBlock,
  instructions: RuntimePresentationAsmInstruction[],
): RuntimePresentationSourceSpan => ({
  file: block.file,
  start_line: block.startLine,
  end_line: instructions.at(-1)?.source_span.end_line ?? block.startLine,
});

const requireExactRoutineBlock = (
  blocks: Map<string, ParsedAsmBlock>,
  blockId: string,
  expected: readonly string[],
  context: string,
  throughFirstReturn = false,
): RuntimePresentationSourceSpan => {
  const block = blocks.get(blockId);
  if (!block) {
    throw new Error(
      `${context} certificate is missing source block ${blockId}`,
    );
  }
  const returnIndex = block.instructions.findIndex(
    (instruction) =>
      instruction.opcode === "ret" && instruction.args.length === 0,
  );
  const instructions =
    throughFirstReturn && returnIndex >= 0
      ? block.instructions.slice(0, returnIndex + 1)
      : block.instructions;
  const actual = instructions.map(instructionSignature);
  const mismatch = Array.from(
    { length: Math.max(actual.length, expected.length) },
    (_, index) => index,
  ).find((index) => actual[index] !== expected[index]);
  if (mismatch !== undefined) {
    const span = instructions[mismatch]?.source_span ??
      instructions.at(-1)?.source_span ?? {
        file: block.file,
        start_line: block.startLine,
        end_line: block.startLine,
      };
    throw new Error(
      `${context} certificate expected ${JSON.stringify(expected[mismatch] ?? "<end>")} ` +
        `but reached ${JSON.stringify(actual[mismatch] ?? "<end>")} at ` +
        `${span.file}:${span.start_line}`,
    );
  }
  return exactRoutineSpan(block, instructions);
};

const requireExactNormalizedRegion = (
  source: LoadedSource,
  start: string,
  end: string,
  expected: readonly string[],
  context: string,
): RuntimePresentationSourceSpan => {
  const normalized = source.lines.map(normalizeAsmLine);
  const startIndex = normalized.indexOf(start);
  const endIndex = normalized.indexOf(end, startIndex + 1);
  if (startIndex < 0 || endIndex < startIndex) {
    throw new Error(
      `${context} certificate is missing exact source region ${start}..${end} in ${source.file}`,
    );
  }
  const actual = normalized.slice(startIndex, endIndex + 1).filter(Boolean);
  const mismatch = Array.from(
    { length: Math.max(actual.length, expected.length) },
    (_, index) => index,
  ).find((index) => actual[index] !== expected[index]);
  if (mismatch !== undefined) {
    throw new Error(
      `${context} certificate expected ${JSON.stringify(expected[mismatch] ?? "<end>")} ` +
        `but reached ${JSON.stringify(actual[mismatch] ?? "<end>")} in ` +
        `${source.file}:${startIndex + 1}-${endIndex + 1}`,
    );
  }
  return {
    file: source.file,
    start_line: startIndex + 1,
    end_line: endIndex + 1,
  };
};

const TRY_LOAD_SAVE_DATA_SOURCE_FILES = {
  save: "engine/menus/save.asm",
  time: "home/time.asm",
  defaults: "data/default_options.asm",
  wram: "ram/wram.asm",
  sram: "ram/sram.asm",
  constants: "constants/misc_constants.asm",
} as const;

const SAVE_OPTION_FIELDS = [
  "wOptions",
  "wSaveFileExists",
  "wTextboxFrame",
  "wTextboxFlags",
  "wGBPrinterBrightness",
  "wOptions2",
  "wOptions + 6",
  "wOptions + 7",
] as const;

const SAVE_BOOT_SLICE_FIELDS = [
  "wStartDay",
  "wStartHour",
  "wStartMinute",
  "wStartSecond",
  "wRTC + 0",
  "wRTC + 1",
  "wRTC + 2",
  "wRTC + 3",
] as const;

const RTC_HRAM_FIELDS = [
  "hRTCSeconds",
  "hRTCMinutes",
  "hRTCHours",
  "hRTCDayLo",
  "hRTCDayHi",
] as const;

const RTC_HARDWARE_REGISTERS = [
  "RAMB_RTC_S",
  "RAMB_RTC_M",
  "RAMB_RTC_H",
  "RAMB_RTC_DL",
  "RAMB_RTC_DH",
] as const;

function certifyTryLoadSaveDataHostEffect(
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationHostEffect {
  const save = loadSource(TRY_LOAD_SAVE_DATA_SOURCE_FILES.save, options);
  const time = loadSource(TRY_LOAD_SAVE_DATA_SOURCE_FILES.time, options);
  const defaults = loadSource(
    TRY_LOAD_SAVE_DATA_SOURCE_FILES.defaults,
    options,
  );
  const wram = loadSource(TRY_LOAD_SAVE_DATA_SOURCE_FILES.wram, options);
  const sram = loadSource(TRY_LOAD_SAVE_DATA_SOURCE_FILES.sram, options);
  const miscConstants = loadSource(
    TRY_LOAD_SAVE_DATA_SOURCE_FILES.constants,
    options,
  );
  const blocks = parseAsmBlocks([save, time]);

  const tryLoadSpan = requireExactRoutineBlock(
    blocks,
    "TryLoadSaveData",
    [
      "xor a",
      "ld [wSaveFileExists], a",
      "call CheckPrimarySaveFile",
      "ld a, [wSaveFileExists]",
      "and a",
      "jr z, .backup",
      "ld a, BANK(sPlayerData)",
      "call OpenSRAM",
      "ld hl, sPlayerData + wStartDay - wPlayerData",
      "ld de, wStartDay",
      "ld bc, 8",
      "call CopyBytes",
      "ld hl, sPlayerData + wStatusFlags - wPlayerData",
      "ld de, wStatusFlags",
      "ld a, [hl]",
      "ld [de], a",
      "call CloseSRAM",
      "ret",
    ],
    "TryLoadSaveData primary path",
  );
  const clearHrtcSpan = requireExactRoutineBlock(
    blocks,
    ".backup@TryLoadSaveData",
    [
      "call CheckBackupSaveFile",
      "ld a, [wSaveFileExists]",
      "and a",
      "jr z, .corrupt",
      "ld a, BANK(sBackupPlayerData)",
      "call OpenSRAM",
      "ld hl, sBackupPlayerData + wStartDay - wPlayerData",
      "ld de, wStartDay",
      "ld bc, 8",
      "call CopyBytes",
      "ld hl, sBackupPlayerData + wStatusFlags - wPlayerData",
      "ld de, wStatusFlags",
      "ld a, [hl]",
      "ld [de], a",
      "call CloseSRAM",
      "ret",
    ],
    "TryLoadSaveData backup path",
  );
  const corruptSpan = requireExactRoutineBlock(
    blocks,
    ".corrupt@TryLoadSaveData",
    [
      "ld hl, DefaultOptions",
      "ld de, wOptions",
      "ld bc, wOptionsEnd - wOptions",
      "call CopyBytes",
      "call ClearClock",
      "ret",
    ],
    "TryLoadSaveData corrupt path",
    true,
  );
  const primaryCheckSpan = requireExactRoutineBlock(
    blocks,
    "CheckPrimarySaveFile",
    [
      "ld a, BANK(sCheckValue1)",
      "call OpenSRAM",
      "ld a, [sCheckValue1]",
      "cp SAVE_CHECK_VALUE_1",
      "jr nz, .nope",
      "ld a, [sCheckValue2]",
      "cp SAVE_CHECK_VALUE_2",
      "jr nz, .nope",
      "ld hl, sOptions",
      "ld de, wOptions",
      "ld bc, wOptionsEnd - wOptions",
      "call CopyBytes",
      "call CloseSRAM",
      "ld a, TRUE",
      "ld [wSaveFileExists], a",
    ],
    "CheckPrimarySaveFile validity and discriminator",
  );
  const primaryReturnSpan = requireExactRoutineBlock(
    blocks,
    ".nope@CheckPrimarySaveFile",
    ["call CloseSRAM", "ret"],
    "CheckPrimarySaveFile return",
  );
  const backupCheckSpan = requireExactRoutineBlock(
    blocks,
    "CheckBackupSaveFile",
    [
      "ld a, BANK(sBackupCheckValue1)",
      "call OpenSRAM",
      "ld a, [sBackupCheckValue1]",
      "cp SAVE_CHECK_VALUE_1",
      "jr nz, .nope",
      "ld a, [sBackupCheckValue2]",
      "cp SAVE_CHECK_VALUE_2",
      "jr nz, .nope",
      "ld hl, sBackupOptions",
      "ld de, wOptions",
      "ld bc, wOptionsEnd - wOptions",
      "call CopyBytes",
      "ld a, $2",
      "ld [wSaveFileExists], a",
    ],
    "CheckBackupSaveFile validity and discriminator",
  );
  const backupReturnSpan = requireExactRoutineBlock(
    blocks,
    ".nope@CheckBackupSaveFile",
    ["call CloseSRAM", "ret"],
    "CheckBackupSaveFile return",
  );

  const clearClockSpan = requireExactRoutineBlock(
    blocks,
    "ClearClock",
    ["call .ClearhRTC", "call SetClock", "ret"],
    "ClearClock",
  );
  requireExactRoutineBlock(
    blocks,
    ".ClearhRTC@ClearClock",
    [
      "xor a",
      "ldh [hRTCSeconds], a",
      "ldh [hRTCMinutes], a",
      "ldh [hRTCHours], a",
      "ldh [hRTCDayLo], a",
      "ldh [hRTCDayHi], a",
      "ret",
    ],
    "ClearClock HRAM reset",
  );
  const setClockSpan = requireExactRoutineBlock(
    blocks,
    "SetClock",
    [
      "ld a, RAMG_SRAM_ENABLE",
      "ld [rRAMG], a",
      "call LatchClock",
      "ld hl, rRAMB",
      "ld de, rRTCREG",
      "ld [hl], RAMB_RTC_DH",
      "ld a, [de]",
      "bit B_RAMB_RTC_DH_HALT, a",
      "ld [de], a",
      "ld [hl], RAMB_RTC_S",
      "ldh a, [hRTCSeconds]",
      "ld [de], a",
      "ld [hl], RAMB_RTC_M",
      "ldh a, [hRTCMinutes]",
      "ld [de], a",
      "ld [hl], RAMB_RTC_H",
      "ldh a, [hRTCHours]",
      "ld [de], a",
      "ld [hl], RAMB_RTC_DL",
      "ldh a, [hRTCDayLo]",
      "ld [de], a",
      "ld [hl], RAMB_RTC_DH",
      "ldh a, [hRTCDayHi]",
      "res B_RAMB_RTC_DH_HALT, a",
      "ld [de], a",
      "call CloseSRAM",
      "ret",
    ],
    "SetClock RTC persistence",
  );

  const optionsSpan = requireExactNormalizedRegion(
    wram,
    "wOptions::",
    "wOptionsEnd::",
    [
      "wOptions::",
      "db",
      "wSaveFileExists:: db",
      "wTextboxFrame::",
      "db",
      "wTextboxFlags::",
      "db",
      "wGBPrinterBrightness::",
      "db",
      "wOptions2::",
      "db",
      "ds 2",
      "wOptionsEnd::",
    ],
    "TryLoadSaveData eight-byte option slice",
  );
  const bootSliceSpan = requireExactNormalizedRegion(
    wram,
    "wStartDay:: db",
    "wRTC:: ds 4",
    [
      "wStartDay:: db",
      "wStartHour:: db",
      "wStartMinute:: db",
      "wStartSecond:: db",
      "wRTC:: ds 4",
    ],
    "TryLoadSaveData eight-byte boot slice",
  );
  requireExactNormalizedRegion(
    wram,
    "wStatusFlags::",
    "wStatusFlags2::",
    ["wStatusFlags::", "db", "wStatusFlags2::"],
    "TryLoadSaveData status byte",
  );
  const normalizedWram = wram.lines.map(normalizeAsmLine);
  const statusStartIndex = normalizedWram.indexOf("wStatusFlags::");
  const statusByteIndex = normalizedWram.indexOf("db", statusStartIndex + 1);
  const statusSpan = {
    file: wram.file,
    start_line: statusStartIndex + 1,
    end_line: statusByteIndex + 1,
  };
  const defaultsSpan = requireExactNormalizedRegion(
    defaults,
    "DefaultOptions:",
    "assert DefaultOptions.End - DefaultOptions == wOptionsEnd - wOptions",
    [
      "DefaultOptions:",
      "db TEXT_DELAY_MED",
      "db FALSE",
      "db FRAME_1",
      "db 1 << FAST_TEXT_DELAY_F",
      "db GBPRINTER_NORMAL",
      "db 1 << MENU_ACCOUNT",
      "db $00",
      "db $00",
      ".End",
      "assert DefaultOptions.End - DefaultOptions == wOptionsEnd - wOptions",
    ],
    "DefaultOptions exact eight-byte option slice",
  );
  const sramSpan = requireExactNormalizedRegion(
    sram,
    "sBackupOptions:: ds wOptionsEnd - wOptions",
    "sCheckValue2:: db",
    [
      "sBackupOptions:: ds wOptionsEnd - wOptions",
      "sBackupCheckValue1:: db",
      "sBackupGameData::",
      "sBackupPlayerData:: ds wPlayerDataEnd - wPlayerData",
      "sBackupCurMapData:: ds wCurMapDataEnd - wCurMapData",
      "sBackupPokemonData:: ds wPokemonDataEnd - wPokemonData",
      "sBackupGameDataEnd::",
      "ds $18a",
      "sBackupChecksum:: dw",
      "sBackupCheckValue2:: db",
      "sStackTop:: dw",
      "if DEF(_DEBUG)",
      "sRTCHaltCheckValue:: dw",
      "sSkipBattle:: db",
      "sDebugTimeCyclesSinceLastCall:: db",
      "sOpenedInvalidSRAM:: db",
      "sIsBugMon:: db",
      "endc",
      'SECTION "Save", SRAM',
      "sOptions:: ds wOptionsEnd - wOptions",
      "sCheckValue1:: db",
      "sGameData::",
      "sPlayerData:: ds wPlayerDataEnd - wPlayerData",
      "sCurMapData:: ds wCurMapDataEnd - wCurMapData",
      "sPokemonData:: ds wPokemonDataEnd - wPokemonData",
      "sGameDataEnd::",
      "ds $18a",
      "sChecksum:: dw",
      "sCheckValue2:: db",
    ],
    "TryLoadSaveData primary and backup SRAM sources",
  );
  const constants = parseAsmConstants([miscConstants]);
  const checkValue1 = constants.get("SAVE_CHECK_VALUE_1");
  const checkValue2 = constants.get("SAVE_CHECK_VALUE_2");
  if (checkValue1 !== 99 || checkValue2 !== 127) {
    throw new Error(
      "TryLoadSaveData check-value certificate requires SAVE_CHECK_VALUE_1=99 and SAVE_CHECK_VALUE_2=127",
    );
  }
  const checkValue1Span = findTokenSpan(
    miscConstants,
    "DEF SAVE_CHECK_VALUE_1 EQU 99",
  );
  const checkValue2Span = findTokenSpan(
    miscConstants,
    "DEF SAVE_CHECK_VALUE_2 EQU 127",
  );
  const constantSpan = {
    file: miscConstants.file,
    start_line: checkValue1Span.start_line,
    end_line: checkValue2Span.end_line,
  };

  const sourceSpan = {
    file: save.file,
    start_line: tryLoadSpan.start_line,
    end_line: corruptSpan.end_line,
  };
  const primaryValiditySpan = {
    file: save.file,
    start_line: primaryCheckSpan.start_line,
    end_line: primaryReturnSpan.end_line,
  };
  const backupValiditySpan = {
    file: save.file,
    start_line: backupCheckSpan.start_line,
    end_line: backupReturnSpan.end_line,
  };
  const defaultValues = [
    "TEXT_DELAY_MED",
    "FALSE",
    "FRAME_1",
    "1 << FAST_TEXT_DELAY_F",
    "GBPRINTER_NORMAL",
    "1 << MENU_ACCOUNT",
    "$00",
    "$00",
  ];
  const instructionRangeSpan = (
    blockId: string,
    startIndex: number,
    endIndex: number,
  ): RuntimePresentationSourceSpan => {
    const block = blocks.get(blockId);
    const start = block?.instructions[startIndex]?.source_span;
    const end = block?.instructions[endIndex]?.source_span;
    if (!start || !end) {
      throw new Error(
        `TryLoadSaveData certificate cannot resolve instruction range ${blockId}[${startIndex}..${endIndex}]`,
      );
    }
    return {
      file: start.file,
      start_line: start.start_line,
      end_line: end.end_line,
    };
  };

  return {
    id: "try_load_save_data",
    call_target: "TryLoadSaveData",
    accepted_call_forms: ["farcall"],
    result: {
      name: "save_source",
      type: "enum",
      domain: [
        {
          id: "none",
          value: 0,
          conditions: [
            { source: "primary", valid: false },
            { source: "backup", valid: false },
          ],
        },
        {
          id: "primary",
          value: 1,
          conditions: [{ source: "primary", valid: true }],
        },
        {
          id: "backup",
          value: 2,
          conditions: [
            { source: "primary", valid: false },
            { source: "backup", valid: true },
          ],
        },
      ],
    },
    validity_checks: [
      {
        source: "primary",
        fields: [
          { name: "sCheckValue1", equals: checkValue1 },
          { name: "sCheckValue2", equals: checkValue2 },
        ],
        source_span: primaryValiditySpan,
      },
      {
        source: "backup",
        fields: [
          { name: "sBackupCheckValue1", equals: checkValue1 },
          { name: "sBackupCheckValue2", equals: checkValue2 },
        ],
        source_span: backupValiditySpan,
      },
    ],
    state_deltas: [
      {
        when: "primary",
        writes: [
          {
            kind: "copy_bytes",
            source: "sOptions",
            target: "wOptions",
            byte_count: 8,
            fields: [...SAVE_OPTION_FIELDS],
            source_span: instructionRangeSpan("CheckPrimarySaveFile", 8, 11),
          },
          {
            kind: "constant_byte",
            target: "wSaveFileExists",
            value: 1,
            source_span: instructionRangeSpan("CheckPrimarySaveFile", 13, 14),
          },
          {
            kind: "copy_bytes",
            source: "sPlayerData + wStartDay - wPlayerData",
            target: "wStartDay",
            byte_count: 8,
            fields: [...SAVE_BOOT_SLICE_FIELDS],
            source_span: instructionRangeSpan("TryLoadSaveData", 8, 11),
          },
          {
            kind: "copy_byte",
            source: "sPlayerData + wStatusFlags - wPlayerData",
            target: "wStatusFlags",
            source_span: instructionRangeSpan("TryLoadSaveData", 12, 15),
          },
        ],
      },
      {
        when: "backup",
        writes: [
          {
            kind: "copy_bytes",
            source: "sBackupOptions",
            target: "wOptions",
            byte_count: 8,
            fields: [...SAVE_OPTION_FIELDS],
            source_span: instructionRangeSpan("CheckBackupSaveFile", 8, 11),
          },
          {
            kind: "constant_byte",
            target: "wSaveFileExists",
            value: 2,
            source_span: instructionRangeSpan("CheckBackupSaveFile", 12, 13),
          },
          {
            kind: "copy_bytes",
            source: "sBackupPlayerData + wStartDay - wPlayerData",
            target: "wStartDay",
            byte_count: 8,
            fields: [...SAVE_BOOT_SLICE_FIELDS],
            source_span: instructionRangeSpan(".backup@TryLoadSaveData", 6, 9),
          },
          {
            kind: "copy_byte",
            source: "sBackupPlayerData + wStatusFlags - wPlayerData",
            target: "wStatusFlags",
            source_span: instructionRangeSpan(
              ".backup@TryLoadSaveData",
              10,
              13,
            ),
          },
        ],
      },
      {
        when: "none",
        writes: [
          {
            kind: "copy_bytes",
            source: "DefaultOptions",
            target: "wOptions",
            byte_count: 8,
            fields: [...SAVE_OPTION_FIELDS],
            values: defaultValues,
            source_span: instructionRangeSpan(".corrupt@TryLoadSaveData", 0, 3),
          },
          {
            kind: "zero_bytes",
            targets: [...RTC_HRAM_FIELDS],
            source_span: clearHrtcSpan,
          },
          {
            kind: "persist_rtc",
            sources: [...RTC_HRAM_FIELDS],
            targets: [...RTC_HARDWARE_REGISTERS],
            clears_halt: true,
            source_span: setClockSpan,
          },
        ],
      },
    ],
    required_consumer: {
      id: "runtime_title_screen.try_load_save_data",
      required: true,
    },
    source_span: sourceSpan,
    implementation_source_spans: [
      sourceSpan,
      primaryValiditySpan,
      backupValiditySpan,
      defaultsSpan,
      optionsSpan,
      bootSliceSpan,
      statusSpan,
      sramSpan,
      constantSpan,
      {
        file: time.file,
        start_line: clearClockSpan.start_line,
        end_line: setClockSpan.end_line,
      },
    ],
  };
}

type RuntimePresentationHostEffectBoundary = {
  accepted_call_forms: RuntimePresentationHostEffectCallForm[];
  certify: (
    options: BuildRuntimeTitlePresentationProgramOptions,
  ) => RuntimePresentationHostEffect;
};

const RUNTIME_PRESENTATION_HOST_EFFECT_BOUNDARIES: Record<
  string,
  RuntimePresentationHostEffectBoundary
> = {
  TryLoadSaveData: {
    accepted_call_forms: ["farcall"],
    certify: certifyTryLoadSaveDataHostEffect,
  },
};

const asmRegionByteSize = (
  source: LoadedSource,
  startLabel: string,
  endLabel: string,
  options: {
    constants?: ReadonlyMap<string, number>;
    unitExpansions?: Readonly<Record<string, string>>;
  } = {},
): number => {
  const normalized = source.lines.map(normalizeAsmLine);
  const startIndex = normalized.indexOf(startLabel);
  const endIndex = normalized.indexOf(endLabel, startIndex + 1);
  if (startIndex < 0 || endIndex < startIndex) {
    throw new Error(
      `Runtime presentation memory region ${startLabel}..${endLabel} is missing from ${source.file}`,
    );
  }
  type UnionFrame = { prefix: number; branches: number[] };
  const unions: UnionFrame[] = [];
  let size = 0;
  for (const rawLine of normalized.slice(startIndex, endIndex + 1)) {
    let line = rawLine;
    const label = line.match(/^(?:[A-Za-z_.][A-Za-z0-9_.@]*|[+-])::?\s*(.*)$/);
    if (label) line = label[1];
    if (!line) continue;
    if (line === "UNION") {
      unions.push({ prefix: size, branches: [] });
      size = 0;
      continue;
    }
    if (line === "NEXTU") {
      const frame = unions.at(-1);
      if (!frame) {
        throw new Error(
          `Runtime presentation memory region ${startLabel} has NEXTU outside UNION`,
        );
      }
      frame.branches.push(size);
      size = 0;
      continue;
    }
    if (line === "ENDU") {
      const frame = unions.pop();
      if (!frame) {
        throw new Error(
          `Runtime presentation memory region ${startLabel} has ENDU outside UNION`,
        );
      }
      frame.branches.push(size);
      size = frame.prefix + Math.max(...frame.branches);
      continue;
    }
    const declaration = line.match(/^(db|dw|ds)(?:\s+(.*))?$/);
    if (!declaration) continue;
    const [, directive, operand = ""] = declaration;
    if (directive === "db" || directive === "dw") {
      const values = operand ? splitInstructionArgs(operand).length : 1;
      size += values * (directive === "dw" ? 2 : 1);
      continue;
    }
    let expression = splitInstructionArgs(operand)[0];
    for (const [unit, expansion] of Object.entries(
      options.unitExpansions ?? {},
    )) {
      expression = expression.replace(
        new RegExp(`\\b${unit}\\b`, "g"),
        expansion,
      );
    }
    size += evaluateAsmInteger(expression, options.constants ?? new Map());
  }
  if (unions.length !== 0) {
    throw new Error(
      `Runtime presentation memory region ${startLabel} has an unterminated UNION`,
    );
  }
  return size;
};

const CLEAR_SPRITE_ANIMS_SOURCE_FILES = {
  implementation: "engine/sprite_anims/core.asm",
  wram: "ram/wram.asm",
  hram: "ram/hram.asm",
  structMacro: "macros/ram.asm",
  spriteConstants: "constants/sprite_anim_constants.asm",
  farcallMacro: "macros/farcall.asm",
  farcallImplementation: "home/farcall.asm",
  restartVectors: "home/header.asm",
  hardware: "constants/hardware.inc",
} as const;

const declarationByteSize = (
  directive: string,
  operand: string,
  constants: ReadonlyMap<string, number>,
): number => {
  if (directive === "db" || directive === "dw") {
    const values = operand ? splitInstructionArgs(operand).length : 1;
    return values * (directive === "dw" ? 2 : 1);
  }
  return evaluateAsmInteger(splitInstructionArgs(operand)[0], constants);
};

const memoryDeclarationAfterLabel = (
  source: LoadedSource,
  label: string,
  constants: ReadonlyMap<string, number>,
): { byte_count: number; source_span: RuntimePresentationSourceSpan } => {
  const normalized = source.lines.map(normalizeAsmLine);
  const escaped = label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const labelExpression = new RegExp(`^${escaped}::?\\s*(.*)$`);
  const labelIndex = normalized.findIndex((line) => labelExpression.test(line));
  if (labelIndex < 0) {
    throw new Error(
      `Runtime presentation memory declaration ${label} is missing from ${source.file}`,
    );
  }
  let declarationIndex = labelIndex;
  let declarationText =
    normalized[labelIndex].match(labelExpression)?.[1] ?? "";
  while (!declarationText && declarationIndex + 1 < normalized.length) {
    declarationIndex += 1;
    declarationText = normalized[declarationIndex];
  }
  const declaration = declarationText.match(/^(db|dw|ds)(?:\s+(.+))?$/);
  if (!declaration) {
    throw new Error(
      `Runtime presentation memory declaration ${label} has unsupported storage ${JSON.stringify(declarationText)} in ${source.file}:${declarationIndex + 1}`,
    );
  }
  return {
    byte_count: declarationByteSize(
      declaration[1],
      declaration[2] ?? "",
      constants,
    ),
    source_span: {
      file: source.file,
      start_line: labelIndex + 1,
      end_line: declarationIndex + 1,
    },
  };
};

const linearMemoryAliasView = (
  source: LoadedSource,
  startLabel: string,
  endLabel: string,
  constants: ReadonlyMap<string, number>,
): {
  byte_count: number;
  labels: string[];
  source_span: RuntimePresentationSourceSpan;
} => {
  const normalized = source.lines.map(normalizeAsmLine);
  const start = normalized.findIndex((line) =>
    new RegExp(`^${startLabel.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}::?`).test(
      line,
    ),
  );
  const end = normalized.findIndex(
    (line, index) =>
      index > start &&
      new RegExp(`^${endLabel.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}::?`).test(
        line,
      ),
  );
  if (start < 0 || end <= start) {
    throw new Error(
      `Runtime presentation memory alias ${startLabel}..${endLabel} is missing from ${source.file}`,
    );
  }
  let byteCount = 0;
  const labels: string[] = [];
  for (let index = start; index < end; index += 1) {
    let line = normalized[index];
    if (!line) continue;
    const label = line.match(/^([A-Za-z_][A-Za-z0-9_]*)::?\s*(.*)$/);
    if (label) {
      labels.push(label[1]);
      line = label[2];
    }
    if (!line || line === "ENDU") continue;
    const declaration = line.match(/^(db|dw|ds)(?:\s+(.+))?$/);
    if (!declaration) {
      throw new Error(
        `Runtime presentation memory alias ${startLabel} has unsupported source ${JSON.stringify(line)} at ${source.file}:${index + 1}`,
      );
    }
    byteCount += declarationByteSize(
      declaration[1],
      declaration[2] ?? "",
      constants,
    );
  }
  return {
    byte_count: byteCount,
    labels,
    source_span: {
      file: source.file,
      start_line: start + 1,
      end_line: end,
    },
  };
};

const spriteAnimationStructMacroLayout = (
  source: LoadedSource,
  constants: ReadonlyMap<string, number>,
): {
  byte_count: number;
  fields: string[];
  source_span: RuntimePresentationSourceSpan;
} => {
  const normalized = source.lines.map(normalizeAsmLine);
  const start = normalized.indexOf("MACRO sprite_anim_struct");
  const end = normalized.indexOf("ENDM", start + 1);
  if (start < 0 || end <= start) {
    throw new Error(
      `sprite_anim_struct source macro is missing from ${source.file}`,
    );
  }
  let byteCount = 0;
  const fields: string[] = [];
  for (let index = start + 1; index < end; index += 1) {
    const line = normalized[index];
    if (!line) continue;
    const declaration = line.match(
      /^\\1([A-Za-z_][A-Za-z0-9_]*)::\s+(db|dw|ds)(?:\s+(.+))?$/,
    );
    if (!declaration || fields.includes(declaration[1])) {
      throw new Error(
        `sprite_anim_struct has unsupported or duplicate field source ${JSON.stringify(line)} at ${source.file}:${index + 1}`,
      );
    }
    fields.push(declaration[1]);
    byteCount += declarationByteSize(
      declaration[2],
      declaration[3] ?? "",
      constants,
    );
  }
  if (fields.length === 0) {
    throw new Error("sprite_anim_struct source macro has no fields");
  }
  return {
    byte_count: byteCount,
    fields,
    source_span: {
      file: source.file,
      start_line: start + 1,
      end_line: end + 1,
    },
  };
};

const spriteAnimationArrayLayout = (
  source: LoadedSource,
  constants: ReadonlyMap<string, number>,
): { count: number; source_span: RuntimePresentationSourceSpan } => {
  const normalized = source.lines.map(normalizeAsmLine);
  const startLabel = normalized.indexOf("wSpriteAnimationStructs::");
  const endLabel = normalized.indexOf(
    "wSpriteAnimationStructsEnd::",
    startLabel + 1,
  );
  if (startLabel < 0 || endLabel <= startLabel) {
    throw new Error(
      "wSpriteAnimationStructs source array is missing from ram/wram.asm",
    );
  }
  const body = normalized.slice(startLabel + 1, endLabel).filter(Boolean);
  const loop = body[0]?.match(/^for n, (.+), (.+)$/);
  if (
    !loop ||
    body.length !== 3 ||
    body[1] !== "wSpriteAnim{d:n}:: sprite_anim_struct wSpriteAnim{d:n}" ||
    body[2] !== "endr"
  ) {
    throw new Error(
      "wSpriteAnimationStructs must be an exact source-derived sprite_anim_struct array",
    );
  }
  const first = evaluateAsmInteger(loop[1], constants);
  const endExclusive = evaluateAsmInteger(loop[2], constants);
  if (endExclusive <= first) {
    throw new Error(
      `wSpriteAnimationStructs has an empty or descending source domain ${first}..${endExclusive}`,
    );
  }
  return {
    count: endExclusive - first,
    source_span: {
      file: source.file,
      start_line: startLabel + 1,
      end_line: endLabel + 1,
    },
  };
};

const exactFarcallInvocation = (
  instruction: RuntimePresentationAsmInstruction,
  target: string,
  options: BuildRuntimeTitlePresentationProgramOptions,
  registerResult: {
    a: number;
    bc: number;
    hl: string;
    de: string;
    flags: {
      zero: boolean;
      subtract: boolean;
      half_carry: boolean;
      carry: boolean;
    };
  } = {
    a: 0,
    bc: 0,
    hl: "wSpriteAnimDataEnd",
    de: "unchanged_by_callee",
    flags: { zero: true, subtract: false, half_carry: false, carry: false },
  },
): Record<string, unknown> => {
  if (
    instruction.opcode !== "farcall" ||
    instruction.args.length !== 1 ||
    instruction.args[0] !== target
  ) {
    throw new Error(
      `${target} presentation subprogram requires the exact farcall form; reached ${instructionSignature(instruction)} at ${instruction.source_span.file}:${instruction.source_span.start_line}`,
    );
  }
  const macro = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.farcallMacro,
    options,
  );
  const implementation = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.farcallImplementation,
    options,
  );
  const vectors = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.restartVectors,
    options,
  );
  const hram = loadSource(CLEAR_SPRITE_ANIMS_SOURCE_FILES.hram, options);
  const wram = loadSource(CLEAR_SPRITE_ANIMS_SOURCE_FILES.wram, options);
  const hardware = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.hardware,
    options,
  );
  const macroSpan = requireExactNormalizedRegion(
    macro,
    "MACRO farcall",
    "ENDM",
    ["MACRO farcall", "ld a, BANK(\\1)", "ld hl, \\1", "rst FarCall", "ENDM"],
    `${target} exact farcall macro expansion`,
  );
  const vectorSpan = requireExactNormalizedRegion(
    vectors,
    "FarCall::",
    'SECTION "rst10", ROM0[$0010]',
    ["FarCall::", "jp FarCall_hl", 'SECTION "rst10", ROM0[$0010]'],
    `${target} FarCall restart vector`,
  );
  const bankSwitchSpan = requireExactNormalizedRegion(
    vectors,
    "Bankswitch::",
    'SECTION "rst18", ROM0[$0018]',
    [
      "Bankswitch::",
      "ldh [hROMBank], a",
      "ld [rROMB], a",
      "ret",
      'SECTION "rst18", ROM0[$0018]',
    ],
    `${target} ROM bank switch`,
  );
  const farcallBlocks = parseAsmBlocks([implementation]);
  const entrySpan = requireExactRoutineBlock(
    farcallBlocks,
    "FarCall_hl",
    [
      "ldh [hTempBank], a",
      "ldh a, [hROMBank]",
      "push af",
      "ldh a, [hTempBank]",
      "rst Bankswitch",
      "call FarCall_JumpToHL",
    ],
    `${target} FarCall_hl entry`,
  );
  const returnSpan = requireExactRoutineBlock(
    farcallBlocks,
    "ReturnFarCall",
    [
      "ld a, b",
      "ld [wFarCallBC], a",
      "ld a, c",
      "ld [wFarCallBC + 1], a",
      "pop bc",
      "ld a, b",
      "rst Bankswitch",
      "ld a, [wFarCallBC]",
      "ld b, a",
      "ld a, [wFarCallBC + 1]",
      "ld c, a",
      "ret",
    ],
    `${target} ReturnFarCall ROM bank restoration`,
  );
  const jumpSpan = requireExactRoutineBlock(
    farcallBlocks,
    "FarCall_JumpToHL",
    ["jp hl"],
    `${target} FarCall target jump`,
  );
  const hTempBankSpan = requireExactNormalizedLine(
    hram,
    "hTempBank:: db",
    `${target} temporary bank storage`,
  );
  const hRomBankSpan = requireExactNormalizedLine(
    hram,
    "hROMBank:: db",
    `${target} current ROM bank storage`,
  );
  const farcallBcSpan = requireExactNormalizedLine(
    wram,
    "wFarCallBC:: dw",
    `${target} callee BC return storage`,
  );
  const romBankRegisterSpan = requireExactNormalizedLine(
    hardware,
    "def rROMB equ $2000",
    `${target} ROM bank hardware register`,
  );
  return {
    call_form: "farcall",
    target,
    target_bank: `BANK(${target})`,
    restores_rom_bank: true,
    preserves_callee_bc: true,
    scratch_writes: [
      { target: "hTempBank", value: `BANK(${target})` },
      {
        target: "wFarCallBC",
        value: [(registerResult.bc >> 8) & 0xff, registerResult.bc & 0xff],
      },
    ],
    register_result: registerResult,
    source_span: instruction.source_span,
    macro_source_span: macroSpan,
    implementation_source_spans: [
      vectorSpan,
      bankSwitchSpan,
      entrySpan,
      returnSpan,
      jumpSpan,
      hTempBankSpan,
      hRomBankSpan,
      farcallBcSpan,
      romBankRegisterSpan,
    ],
  };
};

const compileClearSpriteAnimsCall = (
  instruction: RuntimePresentationAsmInstruction,
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation => {
  const implementation = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.implementation,
    options,
  );
  const wram = loadSource(CLEAR_SPRITE_ANIMS_SOURCE_FILES.wram, options);
  const structMacro = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.structMacro,
    options,
  );
  const spriteConstants = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.spriteConstants,
    options,
  );
  const blocks = parseAsmBlocks([implementation]);
  const implementationSpan = certifyClearSpriteAnimations(blocks);
  const implementationInstructions = handlerFamily(
    "ClearSpriteAnims",
    blocks,
  ).flatMap((block) => block.instructions);
  const constants = parseAsmConstants([spriteConstants]);
  const structMacroLayout = spriteAnimationStructMacroLayout(
    structMacro,
    constants,
  );
  const structArray = spriteAnimationArrayLayout(wram, constants);
  const dictionary = memoryDeclarationAfterLabel(
    wram,
    "wSpriteAnimDict",
    constants,
  );
  const mobileView = linearMemoryAliasView(
    wram,
    "wMobileWRAM",
    "wSpriteAnimCount",
    constants,
  );
  const schedulerByteCount = asmRegionByteSize(
    wram,
    "wSpriteAnimCount:: db",
    "wSpriteAnimDataEnd::",
    { constants },
  );
  const declaredStructLength = constants.get("SPRITEANIMSTRUCT_LENGTH");
  const declaredStructCount = constants.get("NUM_SPRITE_ANIM_STRUCTS");
  const declaredDictionaryEntries = constants.get("NUM_SPRITEANIMDICT_ENTRIES");
  const structLengthSpan = requireExactNormalizedLine(
    spriteConstants,
    "DEF SPRITEANIMSTRUCT_LENGTH EQU _RS",
    "ClearSpriteAnims struct length constant",
  );
  const structCountSpan = requireExactNormalizedLine(
    spriteConstants,
    "DEF NUM_SPRITE_ANIM_STRUCTS EQU 10",
    "ClearSpriteAnims struct count constant",
  );
  const dictionaryCountSpan = requireExactNormalizedLine(
    spriteConstants,
    "DEF NUM_SPRITEANIMDICT_ENTRIES EQU 10",
    "ClearSpriteAnims dictionary count constant",
  );
  if (
    declaredStructLength !== structMacroLayout.byte_count ||
    declaredStructCount !== structArray.count ||
    dictionary.byte_count !== (declaredDictionaryEntries ?? -1) * 2
  ) {
    throw new Error(
      `ClearSpriteAnims wSpriteAnimData layout disagrees with its source constants: struct ${structMacroLayout.byte_count}/${String(declaredStructLength)}, count ${structArray.count}/${String(declaredStructCount)}, dictionary ${dictionary.byte_count}/${String(declaredDictionaryEntries)}`,
    );
  }
  const spriteViewByteCount =
    dictionary.byte_count + structArray.count * structMacroLayout.byte_count;
  if (mobileView.byte_count !== spriteViewByteCount) {
    throw new Error(
      `ClearSpriteAnims wSpriteAnimData mobile alias is ${mobileView.byte_count} bytes but the sprite-animation view is ${spriteViewByteCount}; both UNION branches must be exact`,
    );
  }
  const byteCount = spriteViewByteCount + schedulerByteCount;
  const startSpan = findAsmSymbolDeclarationSpan("wSpriteAnimData", [wram]);
  const endSpan = findAsmSymbolDeclarationSpan("wSpriteAnimDataEnd", [wram]);
  if (!startSpan || !endSpan) {
    throw new Error(
      "ClearSpriteAnims wSpriteAnimData source range has no exact declarations",
    );
  }
  const labels = wram.lines
    .slice(startSpan.start_line - 1, endSpan.end_line)
    .map(normalizeAsmLine)
    .flatMap((line) => {
      const match = line.match(/^([A-Za-z_][A-Za-z0-9_]*)::?/);
      return match ? [match[1]] : [];
    });
  const invocation = exactFarcallInvocation(
    instruction,
    "ClearSpriteAnims",
    options,
  );
  return {
    op: "fill_memory",
    target: "wSpriteAnimData",
    target_end_exclusive: "wSpriteAnimDataEnd",
    byte_count: byteCount,
    value: 0,
    direction: "ascending",
    bank: { select: "wram0", restore: false },
    condition: { source: null, predicate: "always", source_span: null },
    value_source_span: implementationInstructions[2].source_span,
    destination_views: [
      {
        id: "sprite_animation",
        byte_offset: 0,
        byte_count: spriteViewByteCount,
      },
      {
        id: "mobile_union_alias",
        byte_offset: 0,
        byte_count: mobileView.byte_count,
      },
      {
        id: "scheduler_state",
        byte_offset: spriteViewByteCount,
        byte_count: schedulerByteCount,
      },
    ],
    destination_labels: labels,
    implementation_source_span: implementationSpan,
    layout_source_spans: [
      sourceSpanThrough(startSpan, endSpan),
      dictionary.source_span,
      structArray.source_span,
      structMacroLayout.source_span,
      mobileView.source_span,
      structLengthSpan,
      structCountSpan,
      dictionaryCountSpan,
    ],
    invocation,
    source_span: instruction.source_span,
  };
};

const compileEmptySpriteSchedulerCall = (
  instruction: RuntimePresentationAsmInstruction,
  establishedReset: RuntimePresentationOperation,
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation => {
  const resetInvocation =
    establishedReset.invocation &&
    typeof establishedReset.invocation === "object" &&
    !Array.isArray(establishedReset.invocation)
      ? (establishedReset.invocation as Record<string, unknown>)
      : null;
  if (
    establishedReset.op !== "fill_memory" ||
    establishedReset.target !== "wSpriteAnimData" ||
    establishedReset.target_end_exclusive !== "wSpriteAnimDataEnd" ||
    establishedReset.value !== 0 ||
    resetInvocation?.target !== "ClearSpriteAnims" ||
    (
      resetInvocation?.register_result as Record<string, unknown> | undefined
    )?.bc !== 0
  ) {
    throw new Error(
      "PlaySpriteAnimations zero-instance entry has no exact preceding ClearSpriteAnims state and BC=0 proof",
    );
  }

  const implementation = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.implementation,
    options,
  );
  const wram = loadSource(CLEAR_SPRITE_ANIMS_SOURCE_FILES.wram, options);
  const structMacro = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.structMacro,
    options,
  );
  const spriteConstants = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.spriteConstants,
    options,
  );
  const hardware = loadSource(
    CLEAR_SPRITE_ANIMS_SOURCE_FILES.hardware,
    options,
  );
  const blocks = parseAsmBlocks([implementation]);
  const playSpan = requireExactRoutineBlock(
    blocks,
    "PlaySpriteAnimations",
    [
      "push hl",
      "push de",
      "push bc",
      "push af",
      "ld a, LOW(wShadowOAM)",
      "ld [wCurSpriteOAMAddr], a",
      "call DoNextFrameForAllSprites",
      "pop af",
      "pop bc",
      "pop de",
      "pop hl",
      "ret",
    ],
    "PlaySpriteAnimations exact register preservation and OAM cursor",
  );
  const schedulerEntrySpan = requireExactRoutineBlock(
    blocks,
    "DoNextFrameForAllSprites",
    [
      "ld hl, wSpriteAnimationStructs",
      "ld e, NUM_SPRITE_ANIM_STRUCTS",
    ],
    "PlaySpriteAnimations exact struct-array entry",
  );
  const schedulerLoopSpan = requireExactRoutineBlock(
    blocks,
    ".loop@DoNextFrameForAllSprites",
    [
      "ld a, [hl]",
      "and a",
      "jr z, .next",
      "ld c, l",
      "ld b, h",
      "push hl",
      "push de",
      "call DoSpriteAnimFrame",
      "call UpdateAnimFrame",
      "pop de",
      "pop hl",
      "jr c, .done",
    ],
    "PlaySpriteAnimations callback-before-frame scheduler loop",
  );
  const schedulerNextSpan = requireExactRoutineBlock(
    blocks,
    ".next@DoNextFrameForAllSprites",
    [
      "ld bc, SPRITEANIMSTRUCT_LENGTH",
      "add hl, bc",
      "dec e",
      "jr nz, .loop",
      "ld a, [wCurSpriteOAMAddr]",
      "ld l, a",
      "ld h, HIGH(wShadowOAM)",
    ],
    "PlaySpriteAnimations exact slot advance and remaining-OAM address",
  );
  const schedulerClearSpan = requireExactRoutineBlock(
    blocks,
    ".loop2@DoNextFrameForAllSprites",
    [
      "ld a, l",
      "cp LOW(wShadowOAMEnd)",
      "jr nc, .done",
      "xor a",
      "ld [hli], a",
      "jr .loop2",
    ],
    "PlaySpriteAnimations exact remaining OAM clear",
  );
  const schedulerDoneSpan = requireExactRoutineBlock(
    blocks,
    ".done@DoNextFrameForAllSprites",
    ["ret"],
    "PlaySpriteAnimations exact scheduler return",
  );

  const spriteConstantValues = parseAsmConstants([spriteConstants]);
  const hardwareConstants = parseAsmConstants([hardware]);
  const structArray = spriteAnimationArrayLayout(wram, spriteConstantValues);
  const structMacroLayout = spriteAnimationStructMacroLayout(
    structMacro,
    spriteConstantValues,
  );
  const declaredStructSlots = spriteConstantValues.get(
    "NUM_SPRITE_ANIM_STRUCTS",
  );
  const declaredStructLength = spriteConstantValues.get(
    "SPRITEANIMSTRUCT_LENGTH",
  );
  const oamSize = hardwareConstants.get("OAM_SIZE");
  if (
    declaredStructSlots !== 10 ||
    structArray.count !== declaredStructSlots ||
    declaredStructLength !== structMacroLayout.byte_count ||
    oamSize !== 160
  ) {
    throw new Error(
      `PlaySpriteAnimations source layout is not exact: slots ${String(declaredStructSlots)}/${structArray.count}, struct ${String(declaredStructLength)}/${structMacroLayout.byte_count}, OAM ${String(oamSize)}`,
    );
  }
  const cursorDeclarationSpan = requireExactNormalizedLine(
    wram,
    "wCurSpriteOAMAddr:: db",
    "PlaySpriteAnimations OAM cursor declaration",
  );
  const shadowOamSpan = requireExactNormalizedRegion(
    wram,
    "wShadowOAM::",
    "wShadowOAMEnd::",
    [
      "wShadowOAM::",
      "for n, OAM_COUNT",
      "wShadowOAMSprite{02d:n}:: sprite_oam_struct wShadowOAMSprite{02d:n}",
      "endr",
      "wShadowOAMEnd::",
    ],
    "PlaySpriteAnimations shadow OAM array",
  );
  const oamStructSpan = requireExactNormalizedRegion(
    structMacro,
    "MACRO sprite_oam_struct",
    "ENDM",
    [
      "MACRO sprite_oam_struct",
      "\\1YCoord:: db",
      "\\1XCoord:: db",
      "\\1TileID:: db",
      "\\1Attributes:: db",
      "ENDM",
    ],
    "PlaySpriteAnimations four-byte OAM struct",
  );
  const oamSizeSpan = requireExactNormalizedRegion(
    hardware,
    "def OBJ_SIZE rb 0",
    "def OAM_SIZE equ OBJ_SIZE * OAM_COUNT",
    [
      "def OBJ_SIZE rb 0",
      "def OAM_COUNT equ 40",
      "def OAM_SIZE equ OBJ_SIZE * OAM_COUNT",
    ],
    "PlaySpriteAnimations exact 40-entry OAM size",
  );
  const structCountSpan = requireExactNormalizedLine(
    spriteConstants,
    "DEF NUM_SPRITE_ANIM_STRUCTS EQU 10",
    "PlaySpriteAnimations struct count constant",
  );
  const structLengthSpan = requireExactNormalizedLine(
    spriteConstants,
    "DEF SPRITEANIMSTRUCT_LENGTH EQU _RS",
    "PlaySpriteAnimations struct length constant",
  );
  const resetSourceSpan = establishedReset.source_span;
  const callerFlags = {
    zero: false,
    subtract: false,
    half_carry: false,
    carry: false,
  };
  return {
    op: "sprite_scheduler_step",
    instances: [],
    struct_slots: declaredStructSlots,
    callback_before_frame_update: true,
    oam_cursor: {
      target: "wCurSpriteOAMAddr",
      value: "LOW(wShadowOAM)",
    },
    remaining_oam_clear: {
      target: "wShadowOAM",
      target_end_exclusive: "wShadowOAMEnd",
      byte_count: oamSize,
      value: 0,
      direction: "ascending",
    },
    caller_register_state: {
      bc: 0,
      de: "carried_from_scene_setup",
      flags: callerFlags,
    },
    before_host_call: "DelayFrame",
    reset_source_span: resetSourceSpan,
    implementation_source_spans: [
      playSpan,
      schedulerEntrySpan,
      schedulerLoopSpan,
      schedulerNextSpan,
      schedulerClearSpan,
      schedulerDoneSpan,
      structArray.source_span,
      structMacroLayout.source_span,
      cursorDeclarationSpan,
      shadowOamSpan,
      oamStructSpan,
      oamSizeSpan,
      structCountSpan,
      structLengthSpan,
    ],
    invocation: exactFarcallInvocation(
      instruction,
      "PlaySpriteAnimations",
      options,
      {
        a: 0,
        bc: 0,
        hl: "PlaySpriteAnimations",
        de: "unchanged_by_callee",
        flags: callerFlags,
      },
    ),
    source_span: instruction.source_span,
  };
};

const CLEAR_WINDOW_DATA_SOURCE_FILES = {
  menu: "home/menu.asm",
  copy: "home/copy.asm",
  wram: "ram/wram.asm",
} as const;

function certifyClearWindowDataOperations(
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation[] {
  const menu = loadSource(CLEAR_WINDOW_DATA_SOURCE_FILES.menu, options);
  const copy = loadSource(CLEAR_WINDOW_DATA_SOURCE_FILES.copy, options);
  const wram = loadSource(CLEAR_WINDOW_DATA_SOURCE_FILES.wram, options);
  const blocks = parseAsmBlocks([menu, copy]);
  requireExactRoutineBlock(
    blocks,
    "ClearWindowData",
    [
      "ld hl, wMenuMetadata",
      "call .ClearMenuData",
      "ld hl, wMenuHeader",
      "call .ClearMenuData",
      "ld hl, wMenuData",
      "call .ClearMenuData",
      "ld hl, wMoreMenuData",
      "call .ClearMenuData",
      "ldh a, [rWBK]",
      "push af",
      "ld a, BANK(wWindowStack)",
      "ldh [rWBK], a",
      "xor a",
      "ld hl, wWindowStackBottom",
      "ld [hld], a",
      "ld [hld], a",
      "ld a, l",
      "ld [wWindowStackPointer], a",
      "ld a, h",
      "ld [wWindowStackPointer + 1], a",
      "pop af",
      "ldh [rWBK], a",
      "ret",
    ],
    "ClearWindowData memory and bank restoration",
  );
  requireExactRoutineBlock(
    blocks,
    ".ClearMenuData@ClearWindowData",
    [
      "ld bc, wMenuMetadataEnd - wMenuMetadata",
      "assert wMenuMetadataEnd - wMenuMetadata == wMenuHeaderEnd - wMenuHeader",
      "assert wMenuMetadataEnd - wMenuMetadata == wMenuDataEnd - wMenuData",
      "assert wMenuMetadataEnd - wMenuMetadata == wMoreMenuDataEnd - wMoreMenuData",
      "xor a",
      "call ByteFill",
      "ret",
    ],
    "ClearWindowData ClearMenuData helper",
  );
  requireExactRoutineBlock(
    blocks,
    "ByteFill",
    ["inc b", "inc c", "jr .HandleLoop"],
    "ByteFill exact byte-count loop entry",
  );
  requireExactRoutineBlock(
    blocks,
    ".PutByte@ByteFill",
    ["ld [hli], a"],
    "ByteFill byte write",
  );
  requireExactRoutineBlock(
    blocks,
    ".HandleLoop@ByteFill",
    ["dec c", "jr nz, .PutByte", "dec b", "jr nz, .PutByte", "ret"],
    "ByteFill exact byte-count loop",
  );

  const menuRegions = [
    ["wMenuMetadata::", "wMenuMetadataEnd::"],
    ["wMenuHeader::", "wMenuHeaderEnd::"],
    ["wMenuData::", "wMenuDataEnd::"],
    ["wMoreMenuData::", "wMoreMenuDataEnd::"],
  ] as const;
  const regionSizes = menuRegions.map(([start, end]) =>
    asmRegionByteSize(wram, start, end),
  );
  if (regionSizes.some((size) => size !== 16)) {
    throw new Error(
      `ClearWindowData menu regions must each be exactly 16 bytes; reached ${regionSizes.join(", ")}`,
    );
  }
  const stackSize = asmRegionByteSize(
    wram,
    "wWindowStack:: ds $1000 - 1",
    "wWindowStackBottom:: ds 1",
  );
  if (stackSize !== 0x1000) {
    throw new Error(
      `ClearWindowData window stack source region must be exactly 4096 bytes; reached ${stackSize}`,
    );
  }

  const instructionRangeSpan = (
    startIndex: number,
    endIndex: number,
  ): RuntimePresentationSourceSpan => {
    const block = blocks.get("ClearWindowData");
    const start = block?.instructions[startIndex]?.source_span;
    const end = block?.instructions[endIndex]?.source_span;
    if (!start || !end) {
      throw new Error(
        `ClearWindowData certificate cannot resolve instruction range ${startIndex}..${endIndex}`,
      );
    }
    return {
      file: start.file,
      start_line: start.start_line,
      end_line: end.end_line,
    };
  };
  const targets = [
    "wMenuMetadata",
    "wMenuHeader",
    "wMenuData",
    "wMoreMenuData",
  ];
  const operations: RuntimePresentationOperation[] = targets.map(
    (target, index) => ({
      op: "fill_memory",
      target,
      byte_count: regionSizes[index],
      value: 0,
      direction: "ascending",
      bank: { select: "current", restore: false },
      condition: { source: null, predicate: "always", source_span: null },
      source_span: instructionRangeSpan(index * 2, index * 2 + 1),
    }),
  );
  operations.push(
    {
      op: "fill_memory",
      target: "wWindowStackBottom",
      byte_count: 2,
      value: 0,
      direction: "descending",
      bank: { select: "BANK(wWindowStack)", restore: true },
      condition: { source: null, predicate: "always", source_span: null },
      source_span: instructionRangeSpan(8, 21),
    },
    {
      op: "write_memory_word",
      target: "wWindowStackPointer",
      value: "wWindowStackBottom - 2",
      byte_order: "little_endian",
      condition: { source: null, predicate: "always", source_span: null },
      source_span: instructionRangeSpan(16, 19),
    },
  );
  return operations;
}

const CLEAR_BG_PALETTES_SOURCE_FILES = {
  tilemap: "home/tilemap.asm",
  copy: "home/copy.asm",
  delay: "home/delay.asm",
  palettes: "home/palettes.asm",
  vblank: "home/vblank.asm",
  wram: "ram/wram.asm",
  hardware: "constants/hardware.inc",
  gfxMacros: "macros/gfx.asm",
} as const;

function certifyClearBgPalettesOperations(
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation[] {
  const tilemap = loadSource(CLEAR_BG_PALETTES_SOURCE_FILES.tilemap, options);
  const copy = loadSource(CLEAR_BG_PALETTES_SOURCE_FILES.copy, options);
  const delay = loadSource(CLEAR_BG_PALETTES_SOURCE_FILES.delay, options);
  const palettes = loadSource(CLEAR_BG_PALETTES_SOURCE_FILES.palettes, options);
  const vblank = loadSource(CLEAR_BG_PALETTES_SOURCE_FILES.vblank, options);
  const wram = loadSource(CLEAR_BG_PALETTES_SOURCE_FILES.wram, options);
  const hardware = loadSource(CLEAR_BG_PALETTES_SOURCE_FILES.hardware, options);
  const gfxMacros = loadSource(
    CLEAR_BG_PALETTES_SOURCE_FILES.gfxMacros,
    options,
  );
  const blocks = parseAsmBlocks([tilemap, copy, delay, palettes]);
  requireExactRoutineBlock(
    blocks,
    "ClearBGPalettes",
    ["call ClearPalettes"],
    "ClearBGPalettes palette clear call",
  );
  requireExactRoutineBlock(
    blocks,
    "WaitBGMap",
    ["ld a, 1", "ldh [hBGMapMode], a", "ld c, 4", "call DelayFrames", "ret"],
    "ClearBGPalettes WaitBGMap four-frame fallthrough",
  );
  requireExactRoutineBlock(
    blocks,
    "ClearPalettes",
    [
      "ldh a, [hCGB]",
      "and a",
      "jr nz, .cgb",
      "xor a",
      "ldh [rBGP], a",
      "ldh [rOBP0], a",
      "ldh [rOBP1], a",
      "ret",
    ],
    "ClearPalettes DMG register path",
  );
  requireExactRoutineBlock(
    blocks,
    ".cgb@ClearPalettes",
    [
      "ldh a, [rWBK]",
      "push af",
      "ld a, BANK(wBGPals2)",
      "ldh [rWBK], a",
      "ld hl, wBGPals2",
      "ld bc, 16 palettes",
      "ld a, $ff",
      "call ByteFill",
      "pop af",
      "ldh [rWBK], a",
      "ld a, TRUE",
      "ldh [hCGBPalUpdate], a",
      "ret",
    ],
    "ClearPalettes CGB buffers and palette request",
  );
  requireExactRoutineBlock(
    blocks,
    "ByteFill",
    ["inc b", "inc c", "jr .HandleLoop"],
    "ClearPalettes ByteFill loop entry",
  );
  requireExactRoutineBlock(
    blocks,
    ".PutByte@ByteFill",
    ["ld [hli], a"],
    "ClearPalettes ByteFill byte write",
  );
  requireExactRoutineBlock(
    blocks,
    ".HandleLoop@ByteFill",
    ["dec c", "jr nz, .PutByte", "dec b", "jr nz, .PutByte", "ret"],
    "ClearPalettes ByteFill exact byte count",
  );
  requireExactRoutineBlock(
    blocks,
    "DelayFrame",
    ["ld a, 1", "ld [wVBlankOccurred], a"],
    "WaitBGMap one-frame VBlank request",
  );
  requireExactRoutineBlock(
    blocks,
    ".halt@DelayFrame",
    ["halt", "nop", "ld a, [wVBlankOccurred]", "and a", "jr nz, .halt", "ret"],
    "WaitBGMap one-frame VBlank completion wait",
  );
  requireExactRoutineBlock(
    blocks,
    "DelayFrames",
    ["call DelayFrame", "dec c", "jr nz, DelayFrames", "ret"],
    "WaitBGMap exact frame-count loop",
  );
  requireExactRoutineBlock(
    blocks,
    "UpdatePalsIfCGB",
    ["ldh a, [hCGB]", "and a", "ret z"],
    "CGB VBlank palette gate",
  );
  requireExactRoutineBlock(
    blocks,
    "UpdateCGBPals",
    ["ldh a, [hCGBPalUpdate]", "and a", "ret z"],
    "CGB VBlank palette request gate",
  );
  const forceUpdateSpan = requireExactRoutineBlock(
    blocks,
    "ForceUpdateCGBPals",
    [
      "ldh a, [rWBK]",
      "push af",
      "ld a, BANK(wBGPals2)",
      "ldh [rWBK], a",
      "ld hl, wBGPals2",
      "ld a, BGPI_AUTOINC",
      "ldh [rBGPI], a",
      "ld c, LOW(rBGPD)",
      "ld b, 8 / 2",
    ],
    "CGB background palette transfer setup",
  );
  requireExactRoutineBlock(
    blocks,
    ".bgp@ForceUpdateCGBPals",
    [
      "rept (1 palettes) * 2",
      "ld a, [hli]",
      "ldh [c], a",
      "endr",
      "dec b",
      "jr nz, .bgp",
      "ld a, OBPI_AUTOINC",
      "ldh [rOBPI], a",
      "ld c, LOW(rOBPD)",
      "ld b, 8 / 2",
    ],
    "CGB background palette transfer loop",
  );
  const forceUpdateEndSpan = requireExactRoutineBlock(
    blocks,
    ".obp@ForceUpdateCGBPals",
    [
      "rept (1 palettes) * 2",
      "ld a, [hli]",
      "ldh [c], a",
      "endr",
      "dec b",
      "jr nz, .obp",
      "pop af",
      "ldh [rWBK], a",
      "xor a",
      "ldh [hCGBPalUpdate], a",
      "scf",
      "ret",
    ],
    "CGB object palette transfer and request completion",
  );
  const vblankScheduleSpan = requireExactNormalizedRegion(
    vblank,
    "call UpdateBGMapBuffer",
    "call UpdateBGMap",
    [
      "call UpdateBGMapBuffer",
      "jr c, .done",
      "call UpdatePalsIfCGB",
      "jr c, .done",
      "call DMATransfer",
      "jr c, .done",
      "call UpdateBGMap",
    ],
    "VBlank palette-transfer scheduling order",
  );
  requireExactNormalizedRegion(
    gfxMacros,
    'DEF palred EQUS "(1 << B_COLOR_RED) *"',
    'DEF colors EQUS "* COLOR_SIZE"',
    [
      'DEF palred EQUS "(1 << B_COLOR_RED) *"',
      'DEF palgreen EQUS "(1 << B_COLOR_GREEN) *"',
      'DEF palblue EQUS "(1 << B_COLOR_BLUE) *"',
      'DEF palettes EQUS "* PAL_SIZE"',
      'DEF palette EQUS "+ PAL_SIZE *"',
      'DEF color EQUS "+ COLOR_SIZE *"',
      'DEF colors EQUS "* COLOR_SIZE"',
    ],
    "CGB palette unit source macro",
  );
  const constants = parseAsmConstants([hardware]);
  if (
    constants.get("COLOR_SIZE") !== 2 ||
    constants.get("PAL_COLORS") !== 4 ||
    constants.get("PAL_SIZE") !== 8
  ) {
    throw new Error(
      "ClearPalettes source certificate requires two-byte colors and four-color, eight-byte palettes",
    );
  }
  const paletteBufferSize = asmRegionByteSize(
    wram,
    "wBGPals2:: ds 8 palettes",
    "wOBPals2:: ds 8 palettes",
    { constants, unitExpansions: { palettes: "* PAL_SIZE" } },
  );
  if (paletteBufferSize !== 128) {
    throw new Error(
      `ClearPalettes CGB palette buffer must be exactly 128 bytes; reached ${paletteBufferSize}`,
    );
  }

  const instructionRangeSpan = (
    blockId: string,
    startIndex: number,
    endIndex: number,
  ): RuntimePresentationSourceSpan => {
    const block = blocks.get(blockId);
    const start = block?.instructions[startIndex]?.source_span;
    const end = block?.instructions[endIndex]?.source_span;
    if (!start || !end) {
      throw new Error(
        `ClearBGPalettes certificate cannot resolve ${blockId}[${startIndex}..${endIndex}]`,
      );
    }
    return {
      file: start.file,
      start_line: start.start_line,
      end_line: end.end_line,
    };
  };
  const cgbGuardSpan = instructionRangeSpan("ClearPalettes", 0, 2);
  const dmgCondition = {
    source: "hCGB",
    predicate: "zero",
    source_span: cgbGuardSpan,
  };
  const cgbCondition = {
    source: "hCGB",
    predicate: "nonzero",
    source_span: cgbGuardSpan,
  };
  const alwaysCondition = {
    source: null,
    predicate: "always",
    source_span: null,
  };
  const forceUpdateImplementationSpan = {
    file: palettes.file,
    start_line: 3,
    end_line: forceUpdateEndSpan.end_line,
  };

  return [
    {
      op: "write_memory_byte",
      target: "rBGP",
      value: 0,
      address_space: "hardware_register",
      condition: dmgCondition,
      source_span: instructionRangeSpan("ClearPalettes", 3, 4),
    },
    {
      op: "write_memory_byte",
      target: "rOBP0",
      value: 0,
      address_space: "hardware_register",
      condition: dmgCondition,
      source_span: instructionRangeSpan("ClearPalettes", 3, 5),
    },
    {
      op: "write_memory_byte",
      target: "rOBP1",
      value: 0,
      address_space: "hardware_register",
      condition: dmgCondition,
      source_span: instructionRangeSpan("ClearPalettes", 3, 6),
    },
    {
      op: "fill_memory",
      target: "wBGPals2",
      byte_count: paletteBufferSize,
      value: 0xff,
      direction: "ascending",
      bank: { select: "BANK(wBGPals2)", restore: true },
      condition: cgbCondition,
      source_span: instructionRangeSpan(".cgb@ClearPalettes", 0, 9),
    },
    {
      op: "palette_transfer_request",
      condition: cgbCondition,
      request: {
        target: "hCGBPalUpdate",
        queued_value: 1,
        completion_value: 0,
      },
      background: {
        source: "wBGPals2",
        byte_count: paletteBufferSize / 2,
        target: "cgb_background_palette_ram",
        index_register: "rBGPI",
        data_register: "rBGPD",
        autoincrement: true,
      },
      objects: {
        source: "wOBPals2",
        byte_count: paletteBufferSize / 2,
        target: "cgb_object_palette_ram",
        index_register: "rOBPI",
        data_register: "rOBPD",
        autoincrement: true,
      },
      schedule: "vblank",
      source_span: instructionRangeSpan(".cgb@ClearPalettes", 10, 11),
      implementation_source_spans: [
        {
          ...forceUpdateImplementationSpan,
          start_line: Math.min(
            forceUpdateImplementationSpan.start_line,
            forceUpdateSpan.start_line,
          ),
        },
        vblankScheduleSpan,
      ],
    },
    {
      op: "write_memory_byte",
      target: "hBGMapMode",
      value: 1,
      address_space: "hram",
      condition: alwaysCondition,
      source_span: instructionRangeSpan("WaitBGMap", 0, 1),
    },
    {
      op: "wait_frames",
      frames: 4,
      condition: alwaysCondition,
      source_span: instructionRangeSpan("WaitBGMap", 2, 3),
    },
  ];
}

const CLEAR_TILEMAP_SOURCE_FILES = {
  text: "home/text.asm",
  tilemap: "home/tilemap.asm",
  copy: "home/copy.asm",
  delay: "home/delay.asm",
  wram: "ram/wram.asm",
  hardware: "constants/hardware.inc",
  charmap: "constants/charmap.asm",
  coordsMacros: "macros/coords.asm",
} as const;

function certifyClearTilemapOperations(
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation[] {
  const text = loadSource(CLEAR_TILEMAP_SOURCE_FILES.text, options);
  const tilemap = loadSource(CLEAR_TILEMAP_SOURCE_FILES.tilemap, options);
  const copy = loadSource(CLEAR_TILEMAP_SOURCE_FILES.copy, options);
  const delay = loadSource(CLEAR_TILEMAP_SOURCE_FILES.delay, options);
  const wram = loadSource(CLEAR_TILEMAP_SOURCE_FILES.wram, options);
  const hardware = loadSource(CLEAR_TILEMAP_SOURCE_FILES.hardware, options);
  const charmap = loadSource(CLEAR_TILEMAP_SOURCE_FILES.charmap, options);
  const coordsMacros = loadSource(
    CLEAR_TILEMAP_SOURCE_FILES.coordsMacros,
    options,
  );
  const blocks = parseAsmBlocks([text, tilemap, copy, delay]);
  requireExactRoutineBlock(
    blocks,
    "ClearTilemap",
    [
      "hlcoord 0, 0",
      "ld a, ' '",
      "ld bc, wTilemapEnd - wTilemap",
      "call ByteFill",
      "ldh a, [rLCDC]",
      "bit B_LCDC_ENABLE, a",
      "ret z",
      "jp WaitBGMap",
    ],
    "ClearTilemap fill, LCD guard, and WaitBGMap schedule",
  );
  requireExactRoutineBlock(
    blocks,
    "ByteFill",
    ["inc b", "inc c", "jr .HandleLoop"],
    "ClearTilemap ByteFill loop entry",
  );
  requireExactRoutineBlock(
    blocks,
    ".PutByte@ByteFill",
    ["ld [hli], a"],
    "ClearTilemap ByteFill byte write",
  );
  requireExactRoutineBlock(
    blocks,
    ".HandleLoop@ByteFill",
    ["dec c", "jr nz, .PutByte", "dec b", "jr nz, .PutByte", "ret"],
    "ClearTilemap ByteFill exact byte count",
  );
  requireExactRoutineBlock(
    blocks,
    "WaitBGMap",
    ["ld a, 1", "ldh [hBGMapMode], a", "ld c, 4", "call DelayFrames", "ret"],
    "ClearTilemap LCD-enabled WaitBGMap schedule",
  );
  requireExactRoutineBlock(
    blocks,
    "DelayFrame",
    ["ld a, 1", "ld [wVBlankOccurred], a"],
    "ClearTilemap one-frame VBlank request",
  );
  requireExactRoutineBlock(
    blocks,
    ".halt@DelayFrame",
    ["halt", "nop", "ld a, [wVBlankOccurred]", "and a", "jr nz, .halt", "ret"],
    "ClearTilemap one-frame VBlank completion wait",
  );
  requireExactRoutineBlock(
    blocks,
    "DelayFrames",
    ["call DelayFrame", "dec c", "jr nz, DelayFrames", "ret"],
    "ClearTilemap exact four-frame loop",
  );
  requireExactNormalizedRegion(
    coordsMacros,
    "MACRO? hlcoord",
    "ENDM",
    ["MACRO? hlcoord", "coord hl, \\#", "ENDM"],
    "ClearTilemap hlcoord macro",
  );
  requireExactNormalizedRegion(
    coordsMacros,
    "MACRO? coord",
    "ENDM",
    [
      "MACRO? coord",
      "if _NARG < 4",
      "ld \\1, (\\3) * SCREEN_WIDTH + (\\2) + wTilemap",
      "else",
      "ld \\1, (\\3) * SCREEN_WIDTH + (\\2) + \\4",
      "endc",
      "ENDM",
    ],
    "ClearTilemap default coordinate origin",
  );
  const spaceGlyphIndex = charmap.lines.findIndex((line) =>
    /^\s*charmap " ",\s*\$7f\s*(?:;.*)?$/.test(line),
  );
  if (spaceGlyphIndex < 0) {
    throw new Error(
      `ClearTilemap space glyph certificate requires charmap \" \", $7f in ${charmap.file}`,
    );
  }
  const spaceGlyphSpan = {
    file: charmap.file,
    start_line: spaceGlyphIndex + 1,
    end_line: spaceGlyphIndex + 1,
  };
  const constants = parseAsmConstants([hardware]);
  const screenArea = constants.get("SCREEN_AREA");
  const lcdEnableBit = constants.get("B_LCDC_ENABLE");
  if (screenArea !== 360 || lcdEnableBit !== 7) {
    throw new Error(
      `ClearTilemap source constants require SCREEN_AREA=360 and B_LCDC_ENABLE=7; reached ${String(screenArea)} and ${String(lcdEnableBit)}`,
    );
  }
  const tilemapSize = asmRegionByteSize(wram, "wTilemap::", "wTilemapEnd::", {
    constants,
  });
  if (tilemapSize !== screenArea) {
    throw new Error(
      `ClearTilemap tilemap region must be exactly ${screenArea} bytes; reached ${tilemapSize}`,
    );
  }

  const instructionRangeSpan = (
    blockId: string,
    startIndex: number,
    endIndex: number,
  ): RuntimePresentationSourceSpan => {
    const block = blocks.get(blockId);
    const start = block?.instructions[startIndex]?.source_span;
    const end = block?.instructions[endIndex]?.source_span;
    if (!start || !end) {
      throw new Error(
        `ClearTilemap certificate cannot resolve ${blockId}[${startIndex}..${endIndex}]`,
      );
    }
    return {
      file: start.file,
      start_line: start.start_line,
      end_line: end.end_line,
    };
  };
  const alwaysCondition = {
    source: null,
    predicate: "always",
    source_span: null,
  };
  const lcdGuardSpan = instructionRangeSpan("ClearTilemap", 4, 6);
  const lcdEnabledCondition = {
    source: "rLCDC",
    predicate: "bit_set",
    bit: { symbol: "B_LCDC_ENABLE", value: lcdEnableBit },
    source_span: lcdGuardSpan,
  };

  return [
    {
      op: "fill_memory",
      target: "wTilemap",
      byte_count: tilemapSize,
      value: 0x7f,
      direction: "ascending",
      bank: { select: "current", restore: false },
      condition: alwaysCondition,
      source_span: instructionRangeSpan("ClearTilemap", 0, 3),
      value_source_span: spaceGlyphSpan,
    },
    {
      op: "write_memory_byte",
      target: "hBGMapMode",
      value: 1,
      address_space: "hram",
      condition: lcdEnabledCondition,
      source_span: instructionRangeSpan("WaitBGMap", 0, 1),
    },
    {
      op: "wait_frames",
      frames: 4,
      condition: lcdEnabledCondition,
      source_span: instructionRangeSpan("WaitBGMap", 2, 3),
    },
  ];
}

function certifyWaitBgMapOperations(
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation[] {
  const tilemap = loadSource("home/tilemap.asm", options);
  const delay = loadSource("home/delay.asm", options);
  const blocks = parseAsmBlocks([tilemap, delay]);
  requireExactRoutineBlock(
    blocks,
    "WaitBGMap",
    ["ld a, 1", "ldh [hBGMapMode], a", "ld c, 4", "call DelayFrames", "ret"],
    "WaitBGMap exact BG-map request and four-frame schedule",
  );
  requireExactRoutineBlock(
    blocks,
    "DelayFrame",
    ["ld a, 1", "ld [wVBlankOccurred], a"],
    "WaitBGMap one-frame VBlank request",
  );
  requireExactRoutineBlock(
    blocks,
    ".halt@DelayFrame",
    ["halt", "nop", "ld a, [wVBlankOccurred]", "and a", "jr nz, .halt", "ret"],
    "WaitBGMap one-frame VBlank completion wait",
  );
  requireExactRoutineBlock(
    blocks,
    "DelayFrames",
    ["call DelayFrame", "dec c", "jr nz, DelayFrames", "ret"],
    "WaitBGMap exact frame-count loop",
  );
  const waitBlock = blocks.get("WaitBGMap");
  const requestStart = waitBlock?.instructions[0]?.source_span;
  const requestEnd = waitBlock?.instructions[1]?.source_span;
  const waitStart = waitBlock?.instructions[2]?.source_span;
  const waitEnd = waitBlock?.instructions[3]?.source_span;
  if (!requestStart || !requestEnd || !waitStart || !waitEnd) {
    throw new Error("WaitBGMap exact source spans are incomplete");
  }
  const condition = {
    source: null,
    predicate: "always",
    source_span: null,
  };
  return [
    {
      op: "write_memory_byte",
      target: "hBGMapMode",
      value: 1,
      address_space: "hram",
      condition,
      source_span: sourceSpanThrough(requestStart, requestEnd),
    },
    {
      op: "wait_frames",
      frames: 4,
      condition,
      source_span: sourceSpanThrough(waitStart, waitEnd),
    },
  ];
}

type RuntimePresentationSourceOperationBoundary = {
  accepted_call_forms: RuntimePresentationHostEffectCallForm[];
  certify: (
    options: BuildRuntimeTitlePresentationProgramOptions,
  ) => RuntimePresentationOperation[];
};

const RUNTIME_PRESENTATION_SOURCE_OPERATION_BOUNDARIES: Record<
  string,
  RuntimePresentationSourceOperationBoundary
> = {
  ClearWindowData: {
    accepted_call_forms: ["call"],
    certify: certifyClearWindowDataOperations,
  },
  ClearBGPalettes: {
    accepted_call_forms: ["call"],
    certify: certifyClearBgPalettesOperations,
  },
  ClearTilemap: {
    accepted_call_forms: ["call"],
    certify: certifyClearTilemapOperations,
  },
  WaitBGMap: {
    accepted_call_forms: ["call"],
    certify: certifyWaitBgMapOperations,
  },
};

const PRESENTATION_ASM_VALUE_SOURCE_FILES = ["ram/vram.asm"] as const;
const PRESENTATION_HRAM_SOURCE_FILE = "ram/hram.asm";
const PRESENTATION_HARDWARE_SOURCE_FILE = "constants/hardware.inc";

type RuntimePresentationAccumulatorValue = {
  value: number | string;
  value_source_span: RuntimePresentationSourceSpan;
};

const sourceSpanThrough = (
  start: RuntimePresentationSourceSpan,
  end: RuntimePresentationSourceSpan,
): RuntimePresentationSourceSpan => {
  if (start.file !== end.file || end.end_line < start.start_line) {
    throw new Error(
      `Runtime presentation data flow crosses an invalid source range from ${start.file}:${start.start_line} to ${end.file}:${end.end_line}`,
    );
  }
  return {
    file: start.file,
    start_line: start.start_line,
    end_line: end.end_line,
  };
};

const findAsmSymbolDeclarationSpan = (
  symbol: string,
  sources: readonly LoadedSource[],
): RuntimePresentationSourceSpan | null => {
  const escaped = symbol.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const declaration = new RegExp(`^\\s*${escaped}::?\\s*(?:$|[;:])`);
  for (const source of sources) {
    const index = source.lines.findIndex((line) => declaration.test(line));
    if (index >= 0) {
      return {
        file: source.file,
        start_line: index + 1,
        end_line: index + 1,
      };
    }
  }
  return null;
};

const resolvePresentationAccumulatorValue = (
  instruction: RuntimePresentationAsmInstruction,
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationAccumulatorValue | null => {
  if (
    instruction.opcode === "xor" &&
    instruction.args.length === 1 &&
    instruction.args[0] === "a"
  ) {
    return {
      value: 0,
      value_source_span: instruction.source_span,
    };
  }
  if (
    instruction.opcode !== "ld" ||
    instruction.args.length !== 2 ||
    instruction.args[0] !== "a"
  ) {
    return null;
  }

  const operand = instruction.args[1];
  const addressByte = operand.match(
    /^(HIGH|LOW)\(([A-Za-z_.][A-Za-z0-9_.@]*)\)$/,
  );
  if (addressByte) {
    const sources = PRESENTATION_ASM_VALUE_SOURCE_FILES.map((file) =>
      loadSource(file, options),
    );
    const declaration = findAsmSymbolDeclarationSpan(addressByte[2], sources);
    if (!declaration) {
      throw new Error(
        `Runtime presentation accumulator operand ${operand} has no exact source symbol declaration in ${PRESENTATION_ASM_VALUE_SOURCE_FILES.join(", ")}`,
      );
    }
    return {
      value: operand,
      value_source_span: declaration,
    };
  }

  try {
    return {
      value: ((evaluateAsmInteger(operand, new Map()) % 256) + 256) % 256,
      value_source_span: instruction.source_span,
    };
  } catch {
    const constantSources = CONSTANT_SOURCE_FILES.map((file) =>
      loadSource(file, options),
    );
    const constants = parseAsmConstants(constantSources);
    try {
      const value =
        ((evaluateAsmInteger(operand, constants) % 256) + 256) % 256;
      const declaration = constantSources
        .flatMap((source) =>
          source.lines.map((line, index) => ({ source, line, index })),
        )
        .find(({ line }) =>
          new RegExp(
            `^\\s*(?:DEF|def)\\s+${operand.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s+(?:EQU|equ)\\b`,
          ).test(line),
        );
      if (!declaration) return null;
      return {
        value,
        value_source_span: {
          file: declaration.source.file,
          start_line: declaration.index + 1,
          end_line: declaration.index + 1,
        },
      };
    } catch {
      return null;
    }
  }
};

const requireHighMemoryWriteTarget = (
  operand: string,
  options: BuildRuntimeTitlePresentationProgramOptions,
): { target: string; address_space: "hram" | "hardware_register" } => {
  const match = operand.match(/^\[\s*([^\]]+?)\s*\]$/);
  const target = match?.[1]?.trim();
  const baseSymbol = target?.match(/^([A-Za-z_.][A-Za-z0-9_.@]*)/)?.[1];
  if (!target || !baseSymbol) {
    throw new Error(
      `Runtime presentation ldh target ${operand} is not an exact high-memory symbol`,
    );
  }
  const sourceFile = baseSymbol.startsWith("h")
    ? PRESENTATION_HRAM_SOURCE_FILE
    : baseSymbol.startsWith("r")
      ? PRESENTATION_HARDWARE_SOURCE_FILE
      : null;
  if (!sourceFile) {
    throw new Error(
      `Runtime presentation ldh target ${target} is outside high memory`,
    );
  }
  const source = loadSource(sourceFile, options);
  const declaration = baseSymbol.startsWith("h")
    ? findAsmSymbolDeclarationSpan(baseSymbol, [source])
    : source.lines.some((line) =>
        new RegExp(
          `^\\s*(?:DEF|def)\\s+${baseSymbol.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}\\s+(?:EQU|equ)\\b`,
        ).test(line),
      );
  if (!declaration) {
    throw new Error(
      `Runtime presentation ldh target ${target} has no exact source declaration in ${sourceFile}`,
    );
  }
  return {
    target,
    address_space: baseSymbol.startsWith("h") ? "hram" : "hardware_register",
  };
};

const compileAccumulatorHighMemoryWriteRun = (
  instructions: readonly RuntimePresentationAsmInstruction[],
  startIndex: number,
  options: BuildRuntimeTitlePresentationProgramOptions,
): { operations: RuntimePresentationOperation[]; consumed: number } | null => {
  const producer = instructions[startIndex];
  const accumulator = resolvePresentationAccumulatorValue(producer, options);
  if (!accumulator) return null;

  const stores: RuntimePresentationAsmInstruction[] = [];
  for (let index = startIndex + 1; index < instructions.length; index += 1) {
    const candidate = instructions[index];
    if (
      candidate.opcode !== "ldh" ||
      candidate.args.length !== 2 ||
      candidate.args[1] !== "a"
    ) {
      break;
    }
    stores.push(candidate);
  }
  if (stores.length === 0) return null;

  const condition = {
    source: null,
    predicate: "always",
    source_span: null,
  };
  return {
    operations: stores.map((store) => {
      const target = requireHighMemoryWriteTarget(store.args[0], options);
      return {
        op: "write_memory_byte",
        ...target,
        value: accumulator.value,
        condition,
        value_source_span: accumulator.value_source_span,
        source_span: sourceSpanThrough(producer.source_span, store.source_span),
      };
    }),
    consumed: stores.length + 1,
  };
};

const requireFixedWramWriteTarget = (
  operand: string,
  options: BuildRuntimeTitlePresentationProgramOptions,
): {
  target: string;
  address_space: "wram";
  declaration_source_span: RuntimePresentationSourceSpan;
  section_source_span: RuntimePresentationSourceSpan;
} => {
  const match = operand.match(/^\[\s*([^\]]+?)\s*\]$/);
  const target = match?.[1]?.trim();
  const baseSymbol = target?.match(/^(w[A-Za-z0-9_.@]*)$/)?.[1];
  if (!target || !baseSymbol) {
    throw new Error(
      `Runtime presentation direct memory target ${operand} is not an exact WRAM symbol`,
    );
  }
  const wram = loadSource("ram/wram.asm", options);
  const normalized = wram.lines.map(normalizeAsmLine);
  const escaped = baseSymbol.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const declarationIndex = normalized.findIndex((line) =>
    new RegExp(`^${escaped}::?(?:\\s|$)`).test(line),
  );
  if (declarationIndex < 0) {
    throw new Error(
      `Runtime presentation direct memory target ${target} has no exact declaration in ram/wram.asm`,
    );
  }
  let sectionIndex = declarationIndex;
  while (
    sectionIndex >= 0 &&
    !normalized[sectionIndex].startsWith("SECTION ")
  ) {
    sectionIndex -= 1;
  }
  if (
    sectionIndex < 0 ||
    !/^SECTION\s+"[^"]+",\s*WRAM0(?:\[.+\])?$/.test(normalized[sectionIndex])
  ) {
    throw new Error(
      `Runtime presentation direct memory target ${target} is not source-proven fixed WRAM0 storage`,
    );
  }
  return {
    target,
    address_space: "wram",
    declaration_source_span: {
      file: wram.file,
      start_line: declarationIndex + 1,
      end_line: declarationIndex + 1,
    },
    section_source_span: {
      file: wram.file,
      start_line: sectionIndex + 1,
      end_line: sectionIndex + 1,
    },
  };
};

const compileAccumulatorWramWriteRun = (
  instructions: readonly RuntimePresentationAsmInstruction[],
  startIndex: number,
  options: BuildRuntimeTitlePresentationProgramOptions,
): { operations: RuntimePresentationOperation[]; consumed: number } | null => {
  const producer = instructions[startIndex];
  const accumulator = resolvePresentationAccumulatorValue(producer, options);
  if (!accumulator) return null;

  const stores: RuntimePresentationAsmInstruction[] = [];
  for (let index = startIndex + 1; index < instructions.length; index += 1) {
    const candidate = instructions[index];
    if (
      candidate.opcode !== "ld" ||
      candidate.args.length !== 2 ||
      candidate.args[1] !== "a" ||
      !candidate.args[0].startsWith("[")
    ) {
      break;
    }
    stores.push(candidate);
  }
  if (stores.length === 0) return null;

  return {
    operations: stores.map((store) => {
      const target = requireFixedWramWriteTarget(store.args[0], options);
      return {
        op: "write_memory_byte",
        target: target.target,
        address_space: target.address_space,
        value: accumulator.value,
        condition: { source: null, predicate: "always", source_span: null },
        value_source_span: accumulator.value_source_span,
        target_declaration_source_span: target.declaration_source_span,
        target_section_source_span: target.section_source_span,
        source_span: sourceSpanThrough(producer.source_span, store.source_span),
      };
    }),
    consumed: stores.length + 1,
  };
};

const compileAccumulatorWriteSubprogramCall = (
  call: RuntimePresentationAsmInstruction,
  target: string,
  blocks: Map<string, ParsedAsmBlock>,
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation => {
  if (
    call.opcode !== "call" ||
    call.args.length !== 1 ||
    call.args[0] !== target
  ) {
    throw new Error(
      `${target} accumulator-write subprogram requires the exact call form; reached ${instructionSignature(call)} at ${call.source_span.file}:${call.source_span.start_line}`,
    );
  }
  const block = blocks.get(target);
  const instructions = block?.instructions ?? [];
  const writeRun = compileAccumulatorHighMemoryWriteRun(
    instructions,
    0,
    options,
  );
  if (
    !block ||
    !writeRun ||
    writeRun.operations.length !== 1 ||
    writeRun.consumed !== 2 ||
    instructions.length !== 3 ||
    instructionSignature(instructions[2]) !== "ret"
  ) {
    throw new Error(
      `${target} accumulator-write subprogram has unsupported source; expected one resolvable accumulator producer, one high-memory write, and ret, reached ${instructions.map(instructionSignature).join(" -> ")}`,
    );
  }
  const operation = writeRun.operations[0];
  const targetSymbol = String(operation.target);
  const declaration =
    operation.address_space === "hram"
      ? findAsmSymbolDeclarationSpan(targetSymbol, [
          loadSource(PRESENTATION_HRAM_SOURCE_FILE, options),
        ])
      : (() => {
          const hardware = loadSource(
            PRESENTATION_HARDWARE_SOURCE_FILE,
            options,
          );
          const escaped = targetSymbol.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
          const index = hardware.lines.findIndex((line) =>
            new RegExp(`^\\s*(?:DEF|def)\\s+${escaped}\\s+(?:EQU|equ)\\b`).test(
              line,
            ),
          );
          return index < 0
            ? null
            : {
                file: hardware.file,
                start_line: index + 1,
                end_line: index + 1,
              };
        })();
  if (!declaration) {
    throw new Error(
      `${target} accumulator-write target ${targetSymbol} has no exact high-memory declaration`,
    );
  }
  return {
    ...operation,
    target_declaration_source_span: declaration,
    implementation_source_span: exactRoutineSpan(block, instructions),
    invocation: {
      call_form: "call",
      target,
      stack_effect: "push_return_address_then_ret",
      register_result: {
        a: operation.value,
        bc: "unchanged",
        de: "unchanged",
        hl: "unchanged",
        flags:
          instructions[0].opcode === "xor"
            ? {
                zero: true,
                subtract: false,
                half_carry: false,
                carry: false,
              }
            : "unchanged",
      },
      source_span: call.source_span,
    },
    source_span: call.source_span,
  };
};

const compileIncrementMemoryByteSubprogramCall = (
  call: RuntimePresentationAsmInstruction,
  target: string,
  blocks: Map<string, ParsedAsmBlock>,
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation => {
  if (
    call.opcode !== "call" ||
    call.args.length !== 1 ||
    call.args[0] !== target
  ) {
    throw new Error(
      `${target} increment subprogram requires the exact call form; reached ${instructionSignature(call)} at ${call.source_span.file}:${call.source_span.start_line}`,
    );
  }
  const block = blocks.get(target);
  const instructions = block?.instructions ?? [];
  if (
    !block ||
    instructions.length !== 3 ||
    instructions[0].opcode !== "ld" ||
    instructions[0].args.length !== 2 ||
    instructions[0].args[0] !== "hl" ||
    instructionSignature(instructions[1]) !== "inc [hl]" ||
    instructionSignature(instructions[2]) !== "ret"
  ) {
    throw new Error(
      `${target} increment subprogram must source-prove one WRAM byte increment and ret`,
    );
  }
  const memory = requireFixedWramWriteTarget(
    `[${instructions[0].args[1]}]`,
    options,
  );
  return {
    op: "increment_memory_byte",
    target: memory.target,
    address_space: memory.address_space,
    delta: 1,
    wrap: "u8",
    target_declaration_source_span: memory.declaration_source_span,
    target_section_source_span: memory.section_source_span,
    implementation_source_span: exactRoutineSpan(block, instructions),
    invocation: {
      call_form: "call",
      target,
      stack_effect: "push_return_address_then_ret",
      register_result: {
        a: "unchanged",
        bc: "unchanged",
        de: "unchanged",
        hl: memory.target,
        flags: {
          zero: "result_is_zero",
          subtract: false,
          half_carry: "low_nibble_wrapped",
          carry: "unchanged",
        },
      },
      source_span: call.source_span,
    },
    source_span: call.source_span,
  };
};

const requireInstructionSubsequence = (
  blocks: Map<string, ParsedAsmBlock>,
  blockId: string,
  expected: readonly string[],
  context: string,
): RuntimePresentationSourceSpan => {
  const block = blocks.get(blockId);
  if (!block) {
    throw new Error(`${context} is missing source block ${blockId}`);
  }
  const actual = block.instructions.map(instructionSignature);
  const start = actual.findIndex((_, candidate) =>
    expected.every(
      (signature, offset) => actual[candidate + offset] === signature,
    ),
  );
  if (start < 0) {
    throw new Error(
      `${context} expected exact instruction sequence ${expected.join(" -> ")} in ${block.file}:${block.startLine}`,
    );
  }
  return sourceSpanThrough(
    block.instructions[start].source_span,
    block.instructions[start + expected.length - 1].source_span,
  );
};

const requireResourceDirective = (
  source: LoadedSource,
  resourcePath: string,
): RuntimePresentationSourceSpan => {
  const pattern = new RegExp(
    `^\\s*(?:INCBIN|INCLUDE)\\s+"${resourcePath.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}"\\s*(?:;.*)?$`,
  );
  const index = source.lines.findIndex((line) => pattern.test(line));
  if (index < 0) {
    throw new Error(
      `Runtime presentation resource ${resourcePath} is missing from ${source.file}`,
    );
  }
  return { file: source.file, start_line: index + 1, end_line: index + 1 };
};

const requireExactNormalizedLine = (
  source: LoadedSource,
  expected: string,
  context: string,
): RuntimePresentationSourceSpan => {
  const matches = source.lines
    .map(normalizeAsmLine)
    .flatMap((line, index) => (line === expected ? [index] : []));
  if (matches.length !== 1) {
    throw new Error(
      `${context} expected one exact source line ${JSON.stringify(expected)} in ${source.file}, reached ${matches.length}`,
    );
  }
  return {
    file: source.file,
    start_line: matches[0] + 1,
    end_line: matches[0] + 1,
  };
};

const parseByteDataBetweenLabels = (
  source: LoadedSource,
  startLabel: string,
  endLabel: string,
): { bytes: number[]; source_span: RuntimePresentationSourceSpan } => {
  const normalized = source.lines.map(normalizeAsmLine);
  const start = normalized.findIndex((line) => line === startLabel);
  const end = normalized.findIndex((line, index) =>
    index > start ? line === endLabel : false,
  );
  if (start < 0 || end <= start) {
    throw new Error(
      `Runtime presentation byte data ${startLabel}..${endLabel} is missing from ${source.file}`,
    );
  }
  const bytes: number[] = [];
  for (const line of normalized.slice(start + 1, end)) {
    const match = line.match(/^db\s+(.+)$/);
    if (!match) {
      throw new Error(
        `Runtime presentation byte data ${startLabel} has unsupported source ${line} in ${source.file}`,
      );
    }
    for (const operand of splitInstructionArgs(match[1])) {
      const value = evaluateAsmInteger(operand, new Map());
      if (value < 0 || value > 0xff) {
        throw new Error(
          `Runtime presentation byte data ${startLabel} has out-of-range byte ${operand}`,
        );
      }
      bytes.push(value);
    }
  }
  return {
    bytes,
    source_span: {
      file: source.file,
      start_line: start + 1,
      end_line: end + 1,
    },
  };
};

const parseWordTable = (
  source: LoadedSource,
  startLabel: string,
  endDirective: string,
  requireWidth = true,
): {
  entries: string[];
  source_span: RuntimePresentationSourceSpan;
} => {
  const normalized = source.lines.map(normalizeAsmLine);
  const start = normalized.indexOf(startLabel);
  const end = normalized.indexOf(endDirective, start + 1);
  if (start < 0 || end <= start) {
    throw new Error(
      `Runtime presentation word table ${startLabel}..${endDirective} is missing from ${source.file}`,
    );
  }
  const body = normalized.slice(start + 1, end).filter(Boolean);
  if (requireWidth && body[0] !== "table_width 2") {
    throw new Error(
      `Runtime presentation word table ${startLabel} has no exact two-byte width in ${source.file}`,
    );
  }
  const entries: string[] = [];
  const entriesSource = body[0] === "table_width 2" ? body.slice(1) : body;
  for (const line of entriesSource) {
    const match = line.match(/^dw\s+(.+)$/);
    if (!match) {
      throw new Error(
        `Runtime presentation word table ${startLabel} has unsupported source ${line} in ${source.file}`,
      );
    }
    entries.push(...splitInstructionArgs(match[1]));
  }
  return {
    entries,
    source_span: {
      file: source.file,
      start_line: start + 1,
      end_line: end + 1,
    },
  };
};

type RuntimePresentationSourceSubprogramBoundary = {
  accepted_call_forms: RuntimePresentationHostEffectCallForm[];
  certify?: (
    options: BuildRuntimeTitlePresentationProgramOptions,
    controlFlow: RuntimePresentationControlFlow,
  ) => RuntimePresentationCallableSubprogram;
  certify_frontier?: (
    options: BuildRuntimeTitlePresentationProgramOptions,
    controlFlow: RuntimePresentationControlFlow,
  ) => RuntimePresentationEmissionFrontier;
};

function certifySplashScreenSubprogram(
  options: BuildRuntimeTitlePresentationProgramOptions,
  controlFlow: RuntimePresentationControlFlow,
): RuntimePresentationCallableSubprogram {
  const splash = loadSource("engine/movie/splash.asm", options);
  const intro = loadSource("engine/menus/intro_menu.asm", options);
  const gbcOnly = loadSource("engine/movie/gbc_only.asm", options);
  const joypad = loadSource("home/joypad.asm", options);
  const clearSprites = loadSource("home/clear_sprites.asm", options);
  const tilemap = loadSource("home/tilemap.asm", options);
  const delay = loadSource("home/delay.asm", options);
  const miscGraphics = loadSource("gfx/misc.asm", options);
  const fontGraphics = loadSource("gfx/font.asm", options);
  const homeGraphics = loadSource("home/gfx.asm", options);
  const loadFont = loadSource("engine/gfx/load_font.asm", options);
  const cgbLayouts = loadSource("engine/gfx/cgb_layouts.asm", options);
  const sgbLayouts = loadSource("engine/gfx/sgb_layouts.asm", options);
  const color = loadSource("engine/gfx/color.asm", options);
  const sgbPalPackets = loadSource("gfx/sgb/pal_packets.asm", options);
  const sgbBlkPackets = loadSource("gfx/sgb/blk_packets.asm", options);
  const predefPalettes = loadSource("gfx/sgb/predef.pal", options);
  const spriteCore = loadSource("engine/sprite_anims/core.asm", options);
  const wram = loadSource("ram/wram.asm", options);
  const hardware = loadSource("constants/hardware.inc", options);
  const gfxConstants = loadSource("constants/gfx_constants.asm", options);
  const ramConstants = loadSource("constants/ram_constants.asm", options);
  const textConstants = loadSource("constants/text_constants.asm", options);
  const scgbConstants = loadSource("constants/scgb_constants.asm", options);
  const codeMacros = loadSource("macros/code.asm", options);
  const sources = [
    splash,
    intro,
    gbcOnly,
    joypad,
    clearSprites,
    tilemap,
    delay,
    spriteCore,
    homeGraphics,
    loadFont,
    cgbLayouts,
    sgbLayouts,
    color,
  ];
  const blocks = parseAsmBlocks(sources);

  const splashSpan = requireExactRoutineBlock(
    blocks,
    "SplashScreen",
    [
      "ld de, MUSIC_NONE",
      "call PlayMusic",
      "call ClearBGPalettes",
      "call ClearTilemap",
      "ld a, HIGH(vBGMap0)",
      "ldh [hBGMapAddress + 1], a",
      "xor a",
      "ldh [hBGMapAddress], a",
      "ldh [hJoyDown], a",
      "ldh [hSCX], a",
      "ldh [hSCY], a",
      "ld a, SCREEN_HEIGHT_PX",
      "ldh [hWY], a",
      "call WaitBGMap",
      "ld b, SCGB_GAMEFREAK_LOGO",
      "call GetSGBLayout",
      "call SetDefaultBGPAndOBP",
      "ld c, 10",
      "call DelayFrames",
      "callfar Copyright",
      "call WaitBGMap",
      "ld c, 100",
      "call DelayFrames",
      "call ClearTilemap",
      "farcall GBCOnlyScreen",
      "call GameFreakPresentsInit",
    ],
    "SplashScreen ordered setup, copyright, and animation entry",
  );
  const loopSpan = requireExactRoutineBlock(
    blocks,
    ".joy_loop@SplashScreen",
    [
      "call JoyTextDelay",
      "ldh a, [hJoyLast]",
      "and PAD_BUTTONS",
      "jr nz, .pressed_button",
      "ld a, [wJumptableIndex]",
      "bit JUMPTABLE_EXIT_F, a",
      "jr nz, .finish",
      "call GameFreakPresentsScene",
      "farcall PlaySpriteAnimations",
      "call DelayFrame",
      "jr .joy_loop",
    ],
    "SplashScreen input, scene, sprite, VBlank, and repeat order",
  );
  const cancelSpan = requireExactRoutineBlock(
    blocks,
    ".pressed_button@SplashScreen",
    ["call GameFreakPresentsEnd", "scf", "ret"],
    "SplashScreen cancellation carry return",
  );
  const completeSpan = requireExactRoutineBlock(
    blocks,
    ".finish@SplashScreen",
    ["call GameFreakPresentsEnd", "and a", "ret"],
    "SplashScreen completed carry return",
  );
  const initSpan = requireExactRoutineBlock(
    blocks,
    "GameFreakPresentsInit",
    [
      "ld de, GameFreakLogoGFX",
      "ld hl, vTiles2",
      "lb bc, BANK(GameFreakLogoGFX), 28",
      "call Get1bpp",
      "ldh a, [rWBK]",
      "push af",
      "ld a, BANK(wDecompressScratch)",
      "ldh [rWBK], a",
      "ld hl, GameFreakDittoGFX",
      "ld de, wDecompressScratch",
      "ld a, BANK(GameFreakDittoGFX)",
      "call FarDecompress",
      "ld hl, vTiles0",
      "ld de, wDecompressScratch",
      "lb bc, 1, 8 tiles",
      "call Request2bpp",
      "ld hl, vTiles1",
      "ld de, wDecompressScratch + $80 tiles",
      "lb bc, 1, 8 tiles",
      "call Request2bpp",
      "pop af",
      "ldh [rWBK], a",
      "farcall ClearSpriteAnims",
      "depixel 10, 11, 4, 0",
      "ld a, SPRITE_ANIM_OBJ_GAMEFREAK_LOGO",
      "call InitSpriteAnimStruct",
      "ld hl, SPRITEANIMSTRUCT_YOFFSET",
      "add hl, bc",
      "ld [hl], OAM_YCOORD_HIDDEN",
      "ld hl, SPRITEANIMSTRUCT_VAR1",
      "add hl, bc",
      "ld [hl], 96",
      "ld hl, SPRITEANIMSTRUCT_VAR2",
      "add hl, bc",
      "ld [hl], 48",
      "xor a",
      "ld [wJumptableIndex], a",
      "ld [wIntroSceneFrameCounter], a",
      "ld [wIntroSceneTimer], a",
      "ldh [hSCX], a",
      "ldh [hSCY], a",
      "ld a, 1",
      "ldh [hBGMapMode], a",
      "ld a, 144",
      "ldh [hWY], a",
      "lb de, %11100100, %11100100",
      "call DmgToCgbObjPals",
      "ret",
    ],
    "GameFreakPresentsInit graphics, sprite, memory, and palette effects",
  );
  const teardownSpan = requireExactRoutineBlock(
    blocks,
    "GameFreakPresentsEnd",
    [
      "farcall ClearSpriteAnims",
      "call ClearTilemap",
      "call ClearSprites",
      "ld c, 16",
      "call DelayFrames",
      "ret",
    ],
    "GameFreakPresentsEnd exact sprite/tile/OAM teardown and 16-frame wait",
  );
  const sceneWaitSpan = requireExactRoutineBlock(
    blocks,
    "GameFreakPresents_PlaceGameFreak",
    [
      "ld hl, wIntroSceneTimer",
      "ld a, [hl]",
      "cp 32",
      "jr nc, .PlaceGameFreak",
      "inc [hl]",
      "ret",
    ],
    "GameFreak scene-one 32-tick timer",
  );
  const sceneGameFreakSpan = requireExactRoutineBlock(
    blocks,
    ".PlaceGameFreak@GameFreakPresents_PlaceGameFreak",
    [
      "ld [hl], 0",
      "ld hl, .game_freak",
      "decoord 5, 10",
      "ld bc, .end - .game_freak",
      "call CopyBytes",
      "call GameFreakPresents_NextScene",
      "ld de, SFX_GAME_FREAK_PRESENTS",
      "call PlaySFX",
      "ret",
    ],
    "GameFreak text copy and sound",
  );
  const scenePresentsWaitSpan = requireExactRoutineBlock(
    blocks,
    "GameFreakPresents_PlacePresents",
    [
      "ld hl, wIntroSceneTimer",
      "ld a, [hl]",
      "cp 64",
      "jr nc, .place_presents",
      "inc [hl]",
      "ret",
    ],
    "Presents scene 64-tick timer",
  );
  const scenePresentsSpan = requireExactRoutineBlock(
    blocks,
    ".place_presents@GameFreakPresents_PlacePresents",
    [
      "ld [hl], 0",
      "ld hl, .presents",
      "decoord 7, 11",
      "ld bc, .end - .presents",
      "call CopyBytes",
      "call GameFreakPresents_NextScene",
      "ret",
    ],
    "Presents tile copy",
  );
  const sceneFinishSpan = requireExactRoutineBlock(
    blocks,
    "GameFreakPresents_WaitForTimer",
    [
      "ld hl, wIntroSceneTimer",
      "ld a, [hl]",
      "cp 128",
      "jr nc, .finish",
      "inc [hl]",
      "ret",
    ],
    "Final splash scene 128-tick timer",
  );
  const sceneExitSpan = requireExactRoutineBlock(
    blocks,
    ".finish@GameFreakPresents_WaitForTimer",
    ["ld hl, wJumptableIndex", "set JUMPTABLE_EXIT_F, [hl]", "ret"],
    "Final splash scene exit-bit write",
  );

  const gbcEntrySpan = requireExactRoutineBlock(
    blocks,
    "GBCOnlyScreen",
    [
      "ldh a, [hCGB]",
      "and a",
      "ret nz",
      "ld de, MUSIC_NONE",
      "call PlayMusic",
      "call ClearTilemap",
      "ld hl, GBCOnlyGFX",
      "ld de, wGBCOnlyDecompressBuffer",
      "ldh a, [rWBK]",
      "push af",
      "ld a, 0",
      "ldh [rWBK], a",
      "call Decompress",
      "pop af",
      "ldh [rWBK], a",
      "ld de, wGBCOnlyDecompressBuffer",
      "ld hl, vTiles2",
      "lb bc, BANK(GBCOnlyGFX), 84",
      "call Get2bpp",
      "ld de, Font",
      "ld hl, vTiles1",
      "lb bc, BANK(Font), $80",
      "call Get1bpp",
      "call DrawGBCOnlyScreen",
      "call WaitBGMap",
    ],
    "GBCOnlyScreen exact hCGB gate and non-returning DMG display",
  );
  const gbcLoopSpan = requireExactRoutineBlock(
    blocks,
    ".loop@GBCOnlyScreen",
    ["call DelayFrame", "jr .loop"],
    "GBCOnlyScreen non-returning one-VBlank loop",
  );
  requireExactRoutineBlock(
    blocks,
    "DrawGBCOnlyScreen",
    [
      "call DrawGBCOnlyBorder",
      "hlcoord 3, 2",
      "ld b, 14",
      "ld c, 4",
      "ld a, $8",
      "call DrawGBCOnlyGraphic",
      "hlcoord 5, 6",
      "ld b, 10",
      "ld c, 2",
      "ld a, $40",
      "call DrawGBCOnlyGraphic",
      "ld de, GBCOnlyString",
      "hlcoord 1, 10",
      "call PlaceString",
      "ret",
    ],
    "GBCOnlyScreen declarative tilemap composition",
  );
  requireExactRoutineBlock(
    blocks,
    "DrawGBCOnlyBorder",
    [
      "hlcoord 0, 0",
      "ld [hl], 0",
      "inc hl",
      "ld a, 1",
      "call .FillRow",
      "ld [hl], 2",
      "hlcoord 0, 1",
      "ld a, 3",
      "call .FillColumn",
      "hlcoord 19, 1",
      "ld a, 4",
      "call .FillColumn",
      "hlcoord 0, 17",
      "ld [hl], 5",
      "inc hl",
      "ld a, 6",
      "call .FillRow",
      "ld [hl], 7",
      "ret",
    ],
    "GBCOnlyScreen exact border tilemap writes",
  );
  requireExactRoutineBlock(
    blocks,
    "DrawGBCOnlyGraphic",
    ["ld de, SCREEN_WIDTH"],
    "GBCOnlyScreen rectangle row-stride setup",
  );
  requireExactRoutineBlock(
    blocks,
    ".x@DrawGBCOnlyGraphic",
    [
      "ld [hli], a",
      "inc a",
      "dec b",
      "jr nz, .x",
      "pop hl",
      "add hl, de",
      "pop bc",
      "dec c",
      "jr nz, .y",
      "ret",
    ],
    "GBCOnlyScreen rectangle exact tile increment and dimensions",
  );

  const copyrightSpan = requireExactRoutineBlock(
    blocks,
    "Copyright",
    [
      "call ClearTilemap",
      "call LoadFontsExtra",
      "ld de, CopyrightGFX",
      "ld hl, vTiles2 tile $60",
      "lb bc, BANK(CopyrightGFX), 29",
      "call Request2bpp",
      "hlcoord 2, 7",
      "ld de, CopyrightString",
      "jp PlaceString",
    ],
    "Copyright exact graphics transfer and tilemap text placement",
  );
  const loadFontsWrapperSpan = requireExactRoutineBlock(
    blocks,
    "LoadFontsExtra",
    ["farcall _LoadFontsExtra1", "farcall _LoadFontsExtra2", "ret"],
    "LoadFontsExtra exact two-part transfer order",
  );
  const loadFontsExtra1Span = requireExactRoutineBlock(
    blocks,
    "_LoadFontsExtra1",
    [
      "ld de, FontsExtra_SolidBlackGFX",
      "ld hl, vTiles2 tile '■'",
      "lb bc, BANK(FontsExtra_SolidBlackGFX), 1",
      "call Get1bppViaHDMA",
      "ld de, PokegearPhoneIconGFX",
      "ld hl, vTiles2 tile '☎'",
      "lb bc, BANK(PokegearPhoneIconGFX), 1",
      "call Get2bppViaHDMA",
      "ld de, FontExtra + 3 tiles",
      "ld hl, vTiles2 tile '<BOLD_D>'",
      "lb bc, BANK(FontExtra), 22",
      "call Get2bppViaHDMA",
      "jr LoadFrame",
    ],
    "LoadFontsExtra first transfer group",
  );
  const loadFontsExtra2Span = requireExactRoutineBlock(
    blocks,
    "_LoadFontsExtra2",
    [
      "ld de, FontsExtra2_UpArrowGFX",
      "ld hl, vTiles2 tile '▲'",
      "ld b, BANK(FontsExtra2_UpArrowGFX)",
      "ld c, 1",
      "call Get2bppViaHDMA",
      "ret",
    ],
    "LoadFontsExtra second transfer group",
  );
  const loadFrameSpan = requireExactRoutineBlock(
    blocks,
    "LoadFrame",
    [
      "ld a, [wTextboxFrame]",
      "maskbits NUM_FRAMES",
      "ld bc, TEXTBOX_FRAME_TILES * TILE_1BPP_SIZE",
      "ld hl, Frames",
      "call AddNTimes",
      "ld d, h",
      "ld e, l",
      "ld hl, vTiles2 tile '┌'",
      "lb bc, BANK(Frames), TEXTBOX_FRAME_TILES",
      "call Get1bppViaHDMA",
      "ld hl, vTiles2 tile ' '",
      "ld de, TextboxSpaceGFX",
      "lb bc, BANK(TextboxSpaceGFX), 1",
      "call Get1bppViaHDMA",
      "ret",
    ],
    "LoadFrame exact selected frame and space-glyph transfers",
  );
  const maskBitsSpan = requireExactNormalizedRegion(
    codeMacros,
    "MACRO? maskbits",
    "ENDM",
    [
      "MACRO? maskbits",
      'assert 0 < (\\1) && (\\1) <= $100, "bitmask must be 8-bit"',
      "DEF x = (1 << BITWIDTH((\\1) - 1)) - 1",
      "if _NARG == 2",
      "DEF x <<= \\2",
      "endc",
      "and x",
      "ENDM",
    ],
    "LoadFrame source-derived frame selector mask",
  );
  const frameTableSpan = requireExactNormalizedRegion(
    fontGraphics,
    "Frames:",
    "assert_table_length NUM_FRAMES",
    [
      "Frames:",
      "table_width TEXTBOX_FRAME_TILES * TILE_1BPP_SIZE",
      'INCBIN "gfx/frames/1.1bpp"',
      'INCBIN "gfx/frames/2.1bpp"',
      'INCBIN "gfx/frames/3.1bpp"',
      'INCBIN "gfx/frames/4.1bpp"',
      'INCBIN "gfx/frames/5.1bpp"',
      'INCBIN "gfx/frames/6.1bpp"',
      'INCBIN "gfx/frames/7.1bpp"',
      'INCBIN "gfx/frames/8.1bpp"',
      "assert_table_length NUM_FRAMES",
    ],
    "LoadFrame exact source resource table",
  );

  const layoutEntrySpan = requireExactNormalizedRegion(
    tilemap,
    "GetSGBLayout::",
    "predef_jump LoadSGBLayout",
    [
      "GetSGBLayout::",
      "ldh a, [hCGB]",
      "and a",
      "jr nz, .sgb",
      "ldh a, [hSGB]",
      "and a",
      "ret z",
      ".sgb",
      "predef_jump LoadSGBLayout",
    ],
    "GetSGBLayout exact CGB/SGB/DMG gate",
  );
  const cgbDispatchEntrySpan = requireExactRoutineBlock(
    blocks,
    "LoadSGBLayoutCGB",
    [
      "ld a, b",
      "cp SCGB_DEFAULT",
      "jr nz, .not_default",
      "ld a, [wDefaultSGBLayout]",
    ],
    "CGB layout selector entry",
  );
  const cgbDispatchSpan = requireExactRoutineBlock(
    blocks,
    ".not_default@LoadSGBLayoutCGB",
    [
      "cp SCGB_PARTY_MENU_HP_BARS",
      "jp z, CGB_ApplyPartyMenuHPPals",
      "call ResetBGPals",
      "ld l, a",
      "ld h, 0",
      "add hl, hl",
      "ld de, CGBLayoutJumptable",
      "add hl, de",
      "ld a, [hli]",
      "ld h, [hl]",
      "ld l, a",
      "ld de, .done",
      "push de",
      "jp hl",
    ],
    "CGB layout two-byte table dispatch",
  );
  const sgbDispatchEntrySpan = requireExactRoutineBlock(
    blocks,
    "LoadSGBLayout",
    [
      "call CheckCGB",
      "jp nz, LoadSGBLayoutCGB",
      "ld a, b",
      "cp SCGB_DEFAULT",
      "jr nz, .not_default",
      "ld a, [wDefaultSGBLayout]",
    ],
    "SGB layout selector entry",
  );
  const sgbDispatchSpan = requireExactRoutineBlock(
    blocks,
    ".not_default@LoadSGBLayout",
    [
      "cp SCGB_PARTY_MENU_HP_BARS",
      "jp z, SGB_ApplyPartyMenuHPPals",
      "ld l, a",
      "ld h, 0",
      "add hl, hl",
      "ld de, SGBLayoutJumptable",
      "add hl, de",
      "ld a, [hli]",
      "ld h, [hl]",
      "ld l, a",
      "ld de, _LoadSGBLayout_ReturnFromJumptable",
      "push de",
      "jp hl",
    ],
    "SGB layout two-byte table dispatch",
  );
  const sgbReturnSpan = requireExactRoutineBlock(
    blocks,
    "_LoadSGBLayout_ReturnFromJumptable",
    ["push de", "call PushSGBPals", "pop hl", "jp PushSGBPals"],
    "SGB layout two-packet transfer order",
  );
  const cgbGameFreakSpan = requireExactNormalizedRegion(
    cgbLayouts,
    "_CGB_GamefreakLogo:",
    "ret",
    [
      "_CGB_GamefreakLogo:",
      "ld de, wBGPals1",
      "ld a, PREDEFPAL_GAMEFREAK_LOGO_BG",
      "call GetPredefPal",
      "call LoadHLPaletteIntoDE",
      "ld hl, .GamefreakDittoPalette",
      "ld de, wOBPals1",
      "call LoadHLPaletteIntoDE",
      "ld hl, .GamefreakDittoPalette",
      "ld de, wOBPals1 palette 1",
      "call LoadHLPaletteIntoDE",
      "call WipeAttrmap",
      "call ApplyAttrmap",
      "call ApplyPals",
      "ret",
    ],
    "CGB Game Freak palette and attrmap effects",
  );
  const sgbGameFreakSpan = requireExactNormalizedRegion(
    sgbLayouts,
    ".SGB_GamefreakLogo:",
    "ret",
    [
      ".SGB_GamefreakLogo:",
      "ld hl, PalPacket_GamefreakLogo",
      "ld de, BlkPacket_AllPal0",
      "ret",
    ],
    "SGB Game Freak palette and block packets",
  );
  const sgbPalPacketSpan = requireExactNormalizedRegion(
    sgbPalPackets,
    "PalPacket_GamefreakLogo:",
    "PalPacket_Pal01:",
    [
      "PalPacket_GamefreakLogo:",
      "sgb_pal_set GS_INTRO_GAMEFREAK_LOGO, ROUTES, ROUTES, ROUTES",
      "PalPacket_Pal01:",
    ],
    "SGB Game Freak palette packet data",
  );
  const sgbPalMacroSpan = requireExactNormalizedRegion(
    sgbPalPackets,
    "MACRO sgb_pal_set",
    "ENDM",
    [
      "MACRO sgb_pal_set",
      "db (SGB_PAL_SET << 3) + 1",
      "dw PREDEFPAL_\\1, PREDEFPAL_\\2, PREDEFPAL_\\3, PREDEFPAL_\\4",
      "ds 7, 0",
      "ENDM",
    ],
    "SGB palette packet byte encoding",
  );
  const sgbBlockPacketSpan = requireExactNormalizedRegion(
    sgbBlkPackets,
    "BlkPacket_AllPal0:",
    "ds 8, 0",
    [
      "BlkPacket_AllPal0:",
      "attr_blk 1",
      "attr_blk_data %011, 0,0,0, 00,00, 19,17",
      "ds 8, 0",
    ],
    "SGB full-screen palette-zero block packet",
  );
  const getPredefPaletteSpan = requireExactRoutineBlock(
    blocks,
    "GetPredefPal",
    [
      "ld l, a",
      "ld h, 0",
      "add hl, hl",
      "add hl, hl",
      "add hl, hl",
      "ld bc, PredefPals",
      "add hl, bc",
      "ret",
    ],
    "CGB predefined eight-byte palette lookup",
  );
  const loadPaletteSpan = requireExactNormalizedRegion(
    color,
    "LoadHLPaletteIntoDE:",
    "ret",
    [
      "LoadHLPaletteIntoDE:",
      "ldh a, [rWBK]",
      "push af",
      "ld a, BANK(wOBPals1)",
      "ldh [rWBK], a",
      "ld c, 1 palettes",
      ".loop",
      "ld a, [hli]",
      "ld [de], a",
      "inc de",
      "dec c",
      "jr nz, .loop",
      "pop af",
      "ldh [rWBK], a",
      "ret",
    ],
    "CGB exact eight-byte banked palette copy",
  );
  const resetBgPaletteSpan = requireExactNormalizedRegion(
    color,
    "ResetBGPals:",
    "ret",
    [
      "ResetBGPals:",
      "push af",
      "push bc",
      "push de",
      "push hl",
      "ldh a, [rWBK]",
      "push af",
      "ld a, BANK(wBGPals1)",
      "ldh [rWBK], a",
      "ld hl, wBGPals1",
      "ld c, 1 palettes",
      ".loop",
      "ld a, $ff",
      "ld [hli], a",
      "ld [hli], a",
      "ld [hli], a",
      "ld [hli], a",
      "xor a",
      "ld [hli], a",
      "ld [hli], a",
      "ld [hli], a",
      "ld [hli], a",
      "dec c",
      "jr nz, .loop",
      "pop af",
      "ldh [rWBK], a",
      "pop hl",
      "pop de",
      "pop bc",
      "pop af",
      "ret",
    ],
    "CGB layout initial background palette reset",
  );
  const wipeAttrmapSpan = requireExactRoutineBlock(
    blocks,
    "WipeAttrmap",
    [
      "hlcoord 0, 0, wAttrmap",
      "ld bc, SCREEN_AREA",
      "xor a",
      "call ByteFill",
      "ret",
    ],
    "CGB layout exact attrmap clear",
  );
  const applyPalettesSpan = requireExactRoutineBlock(
    blocks,
    "ApplyPals",
    [
      "ld hl, wBGPals1",
      "ld de, wBGPals2",
      "ld bc, 16 palettes",
      "ld a, BANK(wGBCPalettes)",
      "call FarCopyWRAM",
      "ret",
    ],
    "CGB layout exact 16-palette staging copy",
  );
  const applyAttrmapEnabledSpan = requireExactRoutineBlock(
    blocks,
    "ApplyAttrmap",
    [
      "ldh a, [rLCDC]",
      "bit B_LCDC_ENABLE, a",
      "jr z, .UpdateVBank1",
      "ldh a, [hBGMapMode]",
      "push af",
      "ld a, $2",
      "ldh [hBGMapMode], a",
      "call DelayFrame",
      "call DelayFrame",
      "call DelayFrame",
      "call DelayFrame",
      "pop af",
      "ldh [hBGMapMode], a",
      "ret",
    ],
    "CGB layout LCD-enabled four-VBlank attrmap upload",
  );
  const applyAttrmapDisabledSpan = requireExactNormalizedRegion(
    color,
    ".UpdateVBank1:",
    "ret",
    [
      ".UpdateVBank1:",
      "hlcoord 0, 0, wAttrmap",
      "debgcoord 0, 0",
      "ld b, SCREEN_HEIGHT",
      "ld a, $1",
      "ldh [rVBK], a",
      ".row",
      "ld c, SCREEN_WIDTH",
      ".col",
      "ld a, [hli]",
      "ld [de], a",
      "inc de",
      "dec c",
      "jr nz, .col",
      "ld a, TILEMAP_WIDTH - SCREEN_WIDTH",
      "add e",
      "jr nc, .okay",
      "inc d",
      ".okay",
      "ld e, a",
      "dec b",
      "jr nz, .row",
      "ld a, $0",
      "ldh [rVBK], a",
      "ret",
    ],
    "CGB layout LCD-disabled direct VRAM-bank-one attrmap upload",
  );
  const pushSgbPaletteSpan = requireExactRoutineBlock(
    blocks,
    "PushSGBPals",
    [
      "ld a, [wJoypadDisable]",
      "push af",
      "set JOYPAD_DISABLE_SGB_TRANSFER_F, a",
      "ld [wJoypadDisable], a",
      "call _PushSGBPals",
      "pop af",
      "ld [wJoypadDisable], a",
      "ret",
    ],
    "SGB packet transfer joypad-disable preservation",
  );

  const frameConstants = parseAsmConstants([
    ramConstants,
    textConstants,
    hardware,
    gfxConstants,
  ]);
  const frameCount = frameConstants.get("NUM_FRAMES");
  const frameTileCount = frameConstants.get("TEXTBOX_FRAME_TILES");
  const frameTileSize = frameConstants.get("TILE_1BPP_SIZE");
  if (frameCount !== 8 || frameTileCount !== 6 || frameTileSize !== 8) {
    throw new Error(
      `LoadFontsExtra source constants are not exact: NUM_FRAMES=${String(frameCount)}, TEXTBOX_FRAME_TILES=${String(frameTileCount)}, TILE_1BPP_SIZE=${String(frameTileSize)}`,
    );
  }

  const layoutConstants = parseAsmConstants([scgbConstants]);
  const layoutIndex = layoutConstants.get("SCGB_GAMEFREAK_LOGO");
  const layoutCount = layoutConstants.get("NUM_SCGB_LAYOUTS");
  const sgbPaletteIndex = layoutConstants.get(
    "PREDEFPAL_GS_INTRO_GAMEFREAK_LOGO",
  );
  const routesPaletteIndex = layoutConstants.get("PREDEFPAL_ROUTES");
  const cgbPaletteIndex = layoutConstants.get("PREDEFPAL_GAMEFREAK_LOGO_BG");
  const cgbTable = parseWordTable(
    cgbLayouts,
    "CGBLayoutJumptable:",
    "assert_table_length NUM_SCGB_LAYOUTS",
  );
  const sgbTable = parseWordTable(
    sgbLayouts,
    "SGBLayoutJumptable:",
    "assert_table_length NUM_SCGB_LAYOUTS",
  );
  if (
    layoutIndex !== 25 ||
    layoutCount !== 31 ||
    cgbTable.entries.length !== layoutCount ||
    sgbTable.entries.length !== layoutCount ||
    cgbTable.entries[layoutIndex] !== "_CGB_GamefreakLogo" ||
    sgbTable.entries[layoutIndex] !== ".SGB_GamefreakLogo"
  ) {
    throw new Error(
      `SCGB_GAMEFREAK_LOGO layout dispatch is not exact at source index ${String(layoutIndex)}`,
    );
  }
  if (
    !Number.isInteger(sgbPaletteIndex) ||
    !Number.isInteger(routesPaletteIndex) ||
    !Number.isInteger(cgbPaletteIndex)
  ) {
    throw new Error(
      "Game Freak layout palette identifiers are missing from source constants",
    );
  }
  const sgbPaletteDataSpan = requireExactNormalizedLine(
    predefPalettes,
    "RGB 31,31,31, 30,26,16, 16,12,09, 00,00,00",
    "SGB Game Freak predefined palette data",
  );
  const routesPaletteDataSpan = requireExactNormalizedLine(
    predefPalettes,
    "RGB 31,31,31, 22,25,19, 16,21,30, 00,00,00",
    "SGB routes predefined palette data",
  );
  const cgbPaletteDataSpan = requireExactNormalizedLine(
    predefPalettes,
    "RGB 00,00,00, 08,11,11, 21,21,21, 31,31,31",
    "CGB Game Freak predefined background palette data",
  );
  const inputSpan = requireExactRoutineBlock(
    blocks,
    "JoyTextDelay",
    [
      "call GetJoypad",
      "ldh a, [hInMenu]",
      "and a",
      "ldh a, [hJoyPressed]",
      "jr z, .ok",
      "ldh a, [hJoyDown]",
    ],
    "JoyTextDelay exact pressed-versus-held selection",
  );
  const inputOkSpan = requireExactRoutineBlock(
    blocks,
    ".ok@JoyTextDelay",
    [
      "ldh [hJoyLast], a",
      "ldh a, [hJoyPressed]",
      "and a",
      "jr z, .checkframedelay",
      "ld a, 15",
      "ld [wTextDelayFrames], a",
      "ret",
    ],
    "JoyTextDelay pressed input repeat reset",
  );
  const inputDelaySpan = requireExactRoutineBlock(
    blocks,
    ".checkframedelay@JoyTextDelay",
    [
      "ld a, [wTextDelayFrames]",
      "and a",
      "jr z, .restartframedelay",
      "xor a",
      "ldh [hJoyLast], a",
      "ret",
    ],
    "JoyTextDelay repeat suppression",
  );
  const inputRestartSpan = requireExactRoutineBlock(
    blocks,
    ".restartframedelay@JoyTextDelay",
    ["ld a, 5", "ld [wTextDelayFrames], a", "ret"],
    "JoyTextDelay repeat restart",
  );
  const clearOamSpan = requireExactRoutineBlock(
    blocks,
    "ClearSprites",
    ["ld hl, wShadowOAM", "ld b, wShadowOAMEnd - wShadowOAM", "xor a"],
    "ClearSprites exact OAM clear setup",
  );
  requireExactRoutineBlock(
    blocks,
    ".loop@ClearSprites",
    ["ld [hli], a", "dec b", "jr nz, .loop", "ret"],
    "ClearSprites exact OAM clear loop",
  );

  const hardwareConstants = parseAsmConstants([hardware]);
  const ramConstantValues = parseAsmConstants([ramConstants]);
  const screenHeight = hardwareConstants.get("SCREEN_HEIGHT_PX");
  const padButtons = hardwareConstants.get("PAD_BUTTONS");
  const exitBit = ramConstantValues.get("JUMPTABLE_EXIT_F");
  if (screenHeight !== 144 || padButtons !== 0x0f || exitBit !== 7) {
    throw new Error(
      `SplashScreen constants are not exact: SCREEN_HEIGHT_PX=${String(screenHeight)}, PAD_BUTTONS=${String(padButtons)}, JUMPTABLE_EXIT_F=${String(exitBit)}`,
    );
  }
  requireExactNormalizedRegion(
    hardware,
    "def OBJ_SIZE rb 0",
    "def OAM_SIZE equ OBJ_SIZE * OAM_COUNT",
    [
      "def OBJ_SIZE rb 0",
      "def OAM_COUNT equ 40",
      "def OAM_SIZE equ OBJ_SIZE * OAM_COUNT",
    ],
    "ClearSprites exact 40 four-byte OAM entries",
  );
  if (
    hardwareConstants.get("OBJ_SIZE") !== 4 ||
    hardwareConstants.get("OAM_COUNT") !== 40 ||
    hardwareConstants.get("OAM_SIZE") !== 160
  ) {
    throw new Error(
      "ClearSprites source constants do not define exactly 40 four-byte OAM entries",
    );
  }

  const splashProgram = controlFlow.sprite_programs.find(
    (candidate) => candidate.instance === "sprite:engine/movie/splash.asm:93",
  );
  const splashSpriteOperations = controlFlow.sprite_operations.filter(
    (operation) =>
      (operation.op === "sprite_init" &&
        operation.instance === "sprite:engine/movie/splash.asm:93") ||
      (operation.op === "sprite_scheduler_step" &&
        operation.instances.includes("sprite:engine/movie/splash.asm:93")),
  );
  const scheduler = splashSpriteOperations.find(
    (
      operation,
    ): operation is Extract<
      RuntimePresentationSpriteOperation,
      { op: "sprite_scheduler_step" }
    > => operation.op === "sprite_scheduler_step",
  );
  if (!splashProgram || !scheduler || splashSpriteOperations.length !== 2) {
    throw new Error(
      "SplashScreen has no exact single initialized sprite and scheduler program",
    );
  }
  const sceneTable = controlFlow.indirect_tables.find(
    (table) => table.table === ".scenes@GameFreakPresentsScene",
  );
  if (
    !sceneTable?.index_domain ||
    sceneTable.index_domain.values.join(",") !== "0,1,2,3"
  ) {
    throw new Error(
      "SplashScreen scene dispatcher has no exact 0..3 source-derived domain",
    );
  }

  const sceneThreshold = (blockId: string): number => {
    const operand = blocks
      .get(blockId)
      ?.instructions.find((instruction) => instruction.opcode === "cp")
      ?.args[0];
    if (!operand) {
      throw new Error(
        `SplashScreen scene ${blockId} has no exact timer compare`,
      );
    }
    return evaluateAsmInteger(operand, new Map());
  };
  const gameFreakWait = sceneThreshold("GameFreakPresents_PlaceGameFreak");
  const presentsWait = sceneThreshold("GameFreakPresents_PlacePresents");
  const finishWait = sceneThreshold("GameFreakPresents_WaitForTimer");
  const callbackTransitionTick =
    1 +
    (splashProgram.initial_memory.var2 + 2) +
    (gameFreakWait + 1) +
    (presentsWait + 1);
  const gameFreakDispatchTick = callbackTransitionTick + gameFreakWait + 1;
  const presentsDispatchTick = gameFreakDispatchTick + presentsWait + 1;
  const naturalSchedulerTicks = presentsDispatchTick + finishWait + 1;
  if (
    callbackTransitionTick !== 149 ||
    gameFreakDispatchTick !== 182 ||
    presentsDispatchTick !== 247 ||
    naturalSchedulerTicks !== 376
  ) {
    throw new Error(
      `SplashScreen scheduler timeline is not exact: ${callbackTransitionTick}/${gameFreakDispatchTick}/${presentsDispatchTick}/${naturalSchedulerTicks}`,
    );
  }

  const gameFreakBytes = parseByteDataBetweenLabels(
    splash,
    ".game_freak",
    ".end",
  );
  const presentsBytes = parseByteDataBetweenLabels(splash, ".presents", ".end");
  if (
    gameFreakBytes.bytes.join(",") !== "0,1,2,3,13,4,5,3,1,6" ||
    presentsBytes.bytes.join(",") !== "7,8,9,10,11,12"
  ) {
    throw new Error("SplashScreen tile-copy source bytes are not exact");
  }

  const resourceSources = new Map<string, LoadedSource>([
    [splash.file, splash],
    [miscGraphics.file, miscGraphics],
    [fontGraphics.file, fontGraphics],
    [cgbLayouts.file, cgbLayouts],
    [gbcOnly.file, gbcOnly],
    [color.file, color],
  ]);
  const resourceDefinitions: Array<[string, string]> = [
    [miscGraphics.file, "gfx/splash/copyright.2bpp"],
    [splash.file, "gfx/splash/gamefreak_presents.1bpp"],
    [splash.file, "gfx/splash/gamefreak_logo.1bpp"],
    [miscGraphics.file, "gfx/splash/ditto.2bpp.lz"],
    [splash.file, "gfx/splash/ditto_fade.pal"],
    [cgbLayouts.file, "gfx/splash/ditto.pal"],
    [color.file, "gfx/sgb/predef.pal"],
    [gbcOnly.file, "gfx/sgb/gbc_only.2bpp.lz"],
    [fontGraphics.file, "gfx/font/font.1bpp"],
    [fontGraphics.file, "gfx/font/black.1bpp"],
    [fontGraphics.file, "gfx/font/phone_icon.2bpp"],
    [fontGraphics.file, "gfx/font/font_extra.2bpp"],
    [fontGraphics.file, "gfx/font/up_arrow.2bpp"],
    [fontGraphics.file, "gfx/font/space.2bpp"],
    ...Array.from(
      { length: 8 },
      (_, index) =>
        [fontGraphics.file, `gfx/frames/${index + 1}.1bpp`] as [string, string],
    ),
  ];
  const resources = resourceDefinitions.map(([file, resourcePath]) => {
    const includeSource = resourceSources.get(file)!;
    const includeSpan = requireResourceDirective(includeSource, resourcePath);
    return {
      path: resourcePath,
      kind: resourcePath.endsWith(".pal")
        ? ("palette" as const)
        : ("tiles" as const),
      include_source_span: includeSpan,
      data_source_span: includeSpan,
    };
  });

  const mainInstructions = blocks.get("SplashScreen")!.instructions;
  const registerWrites: RuntimePresentationOperation[] = [];
  for (let index = 0; index < mainInstructions.length; index += 1) {
    const run = compileAccumulatorHighMemoryWriteRun(
      mainInstructions,
      index,
      options,
    );
    if (!run) continue;
    registerWrites.push(...run.operations);
    index += run.consumed - 1;
  }
  if (
    registerWrites.map((operation) => operation.target).join(",") !==
    "hBGMapAddress + 1,hBGMapAddress,hJoyDown,hSCX,hSCY,hWY"
  ) {
    throw new Error(
      "SplashScreen register initialization does not exactly cover BG-map/input/scroll/window state",
    );
  }

  const alwaysCondition = {
    source: null,
    predicate: "always",
    source_span: null,
  };
  const sourceLineSpan = (
    source: LoadedSource,
    startLine: number,
    endLine: number = startLine,
  ): RuntimePresentationSourceSpan => ({
    file: source.file,
    start_line: startLine,
    end_line: endLine,
  });
  const waitOperation = (
    frames: number,
    span: RuntimePresentationSourceSpan,
    condition: Record<string, unknown> = alwaysCondition,
  ): RuntimePresentationOperation => ({
    op: "wait_frames",
    frames,
    condition,
    source_span: span,
  });
  const clearBgOperations = certifyClearBgPalettesOperations(options);
  const clearTileOperations = certifyClearTilemapOperations(options);
  const waitBgOperations = certifyWaitBgMapOperations(options);
  const clearSpriteCallFor = (
    entry: "GameFreakPresentsInit" | "GameFreakPresentsEnd",
  ): RuntimePresentationOperation => {
    const calls = (blocks.get(entry)?.instructions ?? []).filter(
      (instruction) =>
        runtimePresentationInstructionTarget(instruction) ===
        "ClearSpriteAnims",
    );
    if (calls.length !== 1) {
      throw new Error(
        `${entry} must reach exactly one source-derived ClearSpriteAnims call`,
      );
    }
    return compileClearSpriteAnimsCall(calls[0], options);
  };
  const initializeSpriteMemoryOperation = clearSpriteCallFor(
    "GameFreakPresentsInit",
  );
  const teardownSpriteMemoryOperation = clearSpriteCallFor(
    "GameFreakPresentsEnd",
  );
  const clearOamOperation: RuntimePresentationOperation = {
    op: "fill_memory",
    target: "wShadowOAM",
    byte_count: 160,
    value: 0,
    direction: "ascending",
    bank: { select: "current", restore: false },
    condition: alwaysCondition,
    source_span: clearOamSpan,
  };

  const resourceTransfers: Array<
    { id: string; source_span: RuntimePresentationSourceSpan } & Record<
      string,
      unknown
    >
  > = [
    {
      id: "font_extra_solid_black",
      resources: ["gfx/font/black.1bpp"],
      encoding: "1bpp",
      target: "vTiles2 tile '■'",
      tile_count: 1,
      source_span: sourceLineSpan(loadFont, 45, 48),
      resource_source_spans: [
        requireResourceDirective(fontGraphics, "gfx/font/black.1bpp"),
      ],
    },
    {
      id: "font_extra_phone_icon",
      resources: ["gfx/font/phone_icon.2bpp"],
      encoding: "2bpp",
      target: "vTiles2 tile '☎'",
      tile_count: 1,
      source_span: sourceLineSpan(loadFont, 49, 52),
      resource_source_spans: [
        requireResourceDirective(fontGraphics, "gfx/font/phone_icon.2bpp"),
      ],
    },
    {
      id: "font_extra_glyphs",
      resources: ["gfx/font/font_extra.2bpp"],
      encoding: "2bpp",
      source: "FontExtra + 3 tiles",
      target: "vTiles2 tile '<BOLD_D>'",
      tile_count: 22,
      source_span: sourceLineSpan(loadFont, 53, 56),
      resource_source_spans: [
        requireResourceDirective(fontGraphics, "gfx/font/font_extra.2bpp"),
      ],
    },
    {
      id: "font_extra_selected_frame",
      resources: Array.from(
        { length: frameCount },
        (_, index) => `gfx/frames/${index + 1}.1bpp`,
      ),
      encoding: "1bpp",
      selector: {
        source: "wTextboxFrame",
        operation: "maskbits NUM_FRAMES",
        mask: frameCount - 1,
        domain: Array.from({ length: frameCount }, (_, index) => index),
        element_stride_bytes: frameTileCount * frameTileSize,
        source_spans: [loadFrameSpan, maskBitsSpan, frameTableSpan],
      },
      target: "vTiles2 tile '┌'",
      tile_count: frameTileCount,
      source_span: sourceLineSpan(loadFont, 74, 84),
      resource_source_spans: Array.from({ length: frameCount }, (_, index) =>
        requireResourceDirective(fontGraphics, `gfx/frames/${index + 1}.1bpp`),
      ),
    },
    {
      id: "font_extra_space",
      resources: ["gfx/font/space.2bpp"],
      encoding: "first_half_1bpp",
      target: "vTiles2 tile ' '",
      tile_count: 1,
      source_span: sourceLineSpan(loadFont, 85, 88),
      resource_source_spans: [
        requireResourceDirective(fontGraphics, "gfx/font/space.2bpp"),
      ],
    },
    {
      id: "font_extra_up_arrow",
      resources: ["gfx/font/up_arrow.2bpp"],
      encoding: "2bpp",
      target: "vTiles2 tile '▲'",
      tile_count: 1,
      source_span: sourceLineSpan(loadFont, 59, 64),
      resource_source_spans: [
        requireResourceDirective(fontGraphics, "gfx/font/up_arrow.2bpp"),
      ],
    },
    {
      id: "copyright_tiles",
      resources: ["gfx/splash/copyright.2bpp"],
      encoding: "2bpp",
      target: "vTiles2 tile $60",
      tile_count: 29,
      source_span: sourceLineSpan(intro, 1318, 1321),
      resource_source_spans: [
        requireResourceDirective(miscGraphics, "gfx/splash/copyright.2bpp"),
      ],
    },
    {
      id: "gamefreak_logo_tiles",
      resources: [
        "gfx/splash/gamefreak_presents.1bpp",
        "gfx/splash/gamefreak_logo.1bpp",
      ],
      encoding: "1bpp",
      target: "vTiles2",
      tile_count: 28,
      source_span: sourceLineSpan(splash, 62, 65),
      resource_source_spans: [
        requireResourceDirective(splash, "gfx/splash/gamefreak_presents.1bpp"),
        requireResourceDirective(splash, "gfx/splash/gamefreak_logo.1bpp"),
      ],
    },
    {
      id: "ditto_decompress",
      resources: ["gfx/splash/ditto.2bpp.lz"],
      encoding: "lz_2bpp",
      target: "wDecompressScratch",
      source_span: sourceLineSpan(splash, 72, 75),
      resource_source_spans: [
        requireResourceDirective(miscGraphics, "gfx/splash/ditto.2bpp.lz"),
      ],
    },
    {
      id: "ditto_tiles_bank_0",
      resources: ["gfx/splash/ditto.2bpp.lz"],
      encoding: "decompressed_2bpp",
      source: "wDecompressScratch",
      target: "vTiles0",
      tile_count: 8,
      source_span: sourceLineSpan(splash, 77, 80),
    },
    {
      id: "ditto_tiles_bank_1",
      resources: ["gfx/splash/ditto.2bpp.lz"],
      encoding: "decompressed_2bpp",
      source: "wDecompressScratch + $80 tiles",
      target: "vTiles1",
      tile_count: 8,
      source_span: sourceLineSpan(splash, 82, 85),
    },
    {
      id: "gbc_only_decompress",
      resources: ["gfx/sgb/gbc_only.2bpp.lz"],
      encoding: "lz_2bpp",
      target: "wGBCOnlyDecompressBuffer",
      condition: { source: "hCGB", predicate: "zero" },
      source_span: sourceLineSpan(gbcOnly, 11, 19),
      resource_source_spans: [
        requireResourceDirective(gbcOnly, "gfx/sgb/gbc_only.2bpp.lz"),
      ],
    },
    {
      id: "gbc_only_tiles",
      resources: ["gfx/sgb/gbc_only.2bpp.lz"],
      encoding: "decompressed_2bpp",
      source: "wGBCOnlyDecompressBuffer",
      target: "vTiles2",
      tile_count: 84,
      condition: { source: "hCGB", predicate: "zero" },
      source_span: sourceLineSpan(gbcOnly, 21, 24),
    },
    {
      id: "gbc_only_font",
      resources: ["gfx/font/font.1bpp"],
      encoding: "1bpp",
      target: "vTiles1",
      tile_count: 128,
      condition: { source: "hCGB", predicate: "zero" },
      source_span: sourceLineSpan(gbcOnly, 26, 29),
      resource_source_spans: [
        requireResourceDirective(fontGraphics, "gfx/font/font.1bpp"),
      ],
    },
  ];

  const tilemapWrites: Array<
    { id: string; source_span: RuntimePresentationSourceSpan } & Record<
      string,
      unknown
    >
  > = [
    {
      id: "copyright_string",
      target: "wTilemap coord 2,7",
      source_program: "data/copyright.asm",
      renderer: "PlaceString",
      source_span: sourceLineSpan(intro, 1322, 1324),
      data_source_span: sourceLineSpan(intro, 1326, 1327),
    },
    {
      id: "game_freak_text",
      target: "wTilemap coord 5,10",
      bytes: gameFreakBytes.bytes,
      dispatch_tick: gameFreakDispatchTick,
      source_span: sceneGameFreakSpan,
      data_source_span: gameFreakBytes.source_span,
    },
    {
      id: "presents_text",
      target: "wTilemap coord 7,11",
      bytes: presentsBytes.bytes,
      dispatch_tick: presentsDispatchTick,
      source_span: scenePresentsSpan,
      data_source_span: presentsBytes.source_span,
    },
    {
      id: "gbc_only_border",
      target: "wTilemap",
      writes: {
        corners: [
          { x: 0, y: 0, tile: 0 },
          { x: 19, y: 0, tile: 2 },
          { x: 0, y: 17, tile: 5 },
          { x: 19, y: 17, tile: 7 },
        ],
        rows: [
          { y: 0, x: 1, width: 18, tile: 1 },
          { y: 17, x: 1, width: 18, tile: 6 },
        ],
        columns: [
          { x: 0, y: 1, height: 16, tile: 3 },
          { x: 19, y: 1, height: 16, tile: 4 },
        ],
      },
      condition: { source: "hCGB", predicate: "zero" },
      source_span: sourceLineSpan(gbcOnly, 63, 107),
    },
    {
      id: "gbc_only_graphics",
      target: "wTilemap",
      rectangles: [
        { x: 3, y: 2, width: 14, height: 4, first_tile: 8 },
        { x: 5, y: 6, width: 10, height: 2, first_tile: 0x40 },
      ],
      condition: { source: "hCGB", predicate: "zero" },
      source_span: sourceLineSpan(gbcOnly, 40, 59),
      implementation_source_span: sourceLineSpan(gbcOnly, 109, 124),
    },
    {
      id: "gbc_only_string",
      target: "wTilemap coord 1,10",
      source_program: "GBCOnlyString",
      renderer: "PlaceString",
      condition: { source: "hCGB", predicate: "zero" },
      source_span: sourceLineSpan(gbcOnly, 57, 59),
      data_source_span: sourceLineSpan(gbcOnly, 126, 130),
    },
  ];

  const animationMemoryOperations: RuntimePresentationOperation[] = [
    initializeSpriteMemoryOperation,
    splashSpriteOperations.find((operation) => operation.op === "sprite_init")!,
    ...[
      ["wJumptableIndex", 0, 103, 104],
      ["wIntroSceneFrameCounter", 0, 103, 105],
      ["wIntroSceneTimer", 0, 103, 106],
      ["hSCX", 0, 103, 107],
      ["hSCY", 0, 103, 108],
      ["hBGMapMode", 1, 109, 110],
      ["hWY", 144, 111, 112],
    ].map(([target, value, start, end]) => ({
      op: "write_memory_byte",
      target,
      value,
      address_space: String(target).startsWith("h") ? "hram" : "wram",
      condition: alwaysCondition,
      value_source_span: sourceLineSpan(splash, Number(start), Number(start)),
      source_span: sourceLineSpan(splash, Number(start), Number(end)),
    })),
    {
      op: "set_dmg_palette",
      target: "object_palettes",
      values: [0xe4, 0xe4],
      condition: { source: "hCGB", predicate: "system_dependent" },
      source_span: sourceLineSpan(splash, 113, 114),
      implementation_source_span: sourceLineSpan(tilemap, 146, 166),
    },
  ];
  const frameWait = waitOperation(1, sourceLineSpan(splash, 48));
  const paletteLayoutOperation: RuntimePresentationOperation = {
    op: "apply_palette_layout",
    layout: {
      symbol: "SCGB_GAMEFREAK_LOGO",
      value: layoutIndex,
    },
    selector: {
      register: "b",
      dispatch_width_bytes: 2,
      cgb_table: {
        source: "CGBLayoutJumptable",
        entry: cgbTable.entries[layoutIndex],
        index_domain: [layoutIndex],
        source_span: cgbTable.source_span,
      },
      sgb_table: {
        source: "SGBLayoutJumptable",
        entry: sgbTable.entries[layoutIndex],
        index_domain: [layoutIndex],
        source_span: sgbTable.source_span,
      },
    },
    branches: [
      {
        id: "dmg_without_sgb",
        condition: {
          all: [
            { source: "hCGB", predicate: "zero" },
            { source: "hSGB", predicate: "zero" },
          ],
        },
        operations: [],
        source_span: layoutEntrySpan,
      },
      {
        id: "sgb",
        condition: {
          all: [
            { source: "hCGB", predicate: "zero" },
            { source: "hSGB", predicate: "nonzero" },
          ],
        },
        operations: [
          {
            op: "sgb_packet_transfer",
            order: 0,
            packet: "PalPacket_GamefreakLogo",
            command: "SGB_PAL_SET",
            palettes: [
              {
                symbol: "PREDEFPAL_GS_INTRO_GAMEFREAK_LOGO",
                value: sgbPaletteIndex,
                resource: "gfx/sgb/predef.pal",
                data_source_span: sgbPaletteDataSpan,
              },
              ...Array.from({ length: 3 }, () => ({
                symbol: "PREDEFPAL_ROUTES",
                value: routesPaletteIndex,
                resource: "gfx/sgb/predef.pal",
                data_source_span: routesPaletteDataSpan,
              })),
            ],
            packet_byte_count: 16,
            temporarily_sets: "JOYPAD_DISABLE_SGB_TRANSFER_F",
            restores: "wJoypadDisable",
            source_span: sgbPalPacketSpan,
            encoding_source_span: sgbPalMacroSpan,
          },
          {
            op: "sgb_packet_transfer",
            order: 1,
            packet: "BlkPacket_AllPal0",
            command: "SGB_ATTR_BLK",
            regions: [
              {
                mask: "%011",
                palettes: [0, 0, 0],
                bounds: { x1: 0, y1: 0, x2: 19, y2: 17 },
              },
            ],
            packet_byte_count: 16,
            temporarily_sets: "JOYPAD_DISABLE_SGB_TRANSFER_F",
            restores: "wJoypadDisable",
            source_span: sgbBlockPacketSpan,
          },
        ],
        source_span: sgbGameFreakSpan,
      },
      {
        id: "cgb",
        condition: { source: "hCGB", predicate: "nonzero" },
        operations: [
          {
            op: "write_memory_bytes",
            target: "wBGPals1 palette 0",
            values: [0xff, 0xff, 0xff, 0xff, 0, 0, 0, 0],
            bank: { select: "BANK(wBGPals1)", restore: true },
            source_span: resetBgPaletteSpan,
          },
          {
            op: "copy_palette",
            resource: "gfx/sgb/predef.pal",
            palette: {
              symbol: "PREDEFPAL_GAMEFREAK_LOGO_BG",
              value: cgbPaletteIndex,
            },
            source_palette_index: cgbPaletteIndex,
            source_byte_count: 8,
            target: "wBGPals1 palette 0",
            bank: { select: "BANK(wOBPals1)", restore: true },
            data_source_span: cgbPaletteDataSpan,
            source_span: sourceSpanThrough(
              getPredefPaletteSpan,
              loadPaletteSpan,
            ),
          },
          ...[0, 1].map((palette) => ({
            op: "copy_palette",
            resource: "gfx/splash/ditto.pal",
            source_byte_count: 8,
            target: `wOBPals1 palette ${palette}`,
            bank: { select: "BANK(wOBPals1)", restore: true },
            data_source_span: requireResourceDirective(
              cgbLayouts,
              "gfx/splash/ditto.pal",
            ),
            source_span: cgbGameFreakSpan,
          })),
          {
            op: "fill_memory",
            target: "wAttrmap",
            byte_count: 20 * 18,
            value: 0,
            direction: "ascending",
            bank: { select: "current", restore: false },
            source_span: wipeAttrmapSpan,
          },
          {
            op: "upload_attrmap",
            source: "wAttrmap",
            target: "vBGMap2",
            width: 20,
            height: 18,
            target_stride: 32,
            lcd_enabled: {
              schedule: "hBGMapMode=2",
              waits: 4,
              restores: "hBGMapMode",
              source_span: applyAttrmapEnabledSpan,
            },
            lcd_disabled: {
              vram_bank: 1,
              restores_vram_bank: 0,
              source_span: applyAttrmapDisabledSpan,
            },
            source_span: sourceSpanThrough(
              applyAttrmapEnabledSpan,
              applyAttrmapDisabledSpan,
            ),
          },
          {
            op: "copy_memory",
            source: "wBGPals1",
            target: "wBGPals2",
            byte_count: 16 * 8,
            destination_bank: "BANK(wGBCPalettes)",
            restores_bank: true,
            source_span: applyPalettesSpan,
          },
        ],
        source_span: cgbGameFreakSpan,
      },
    ],
    source_span: sourceLineSpan(splash, 20, 21),
    implementation_source_spans: [
      layoutEntrySpan,
      cgbDispatchEntrySpan,
      cgbDispatchSpan,
      sgbDispatchEntrySpan,
      sgbDispatchSpan,
      sgbReturnSpan,
      cgbGameFreakSpan,
      sgbGameFreakSpan,
      sgbPalPacketSpan,
      sgbPalMacroSpan,
      sgbBlockPacketSpan,
      getPredefPaletteSpan,
      loadPaletteSpan,
      resetBgPaletteSpan,
      wipeAttrmapSpan,
      applyPalettesSpan,
      applyAttrmapEnabledSpan,
      applyAttrmapDisabledSpan,
      pushSgbPaletteSpan,
    ],
  };
  const phases: RuntimePresentationCallableSubprogram["phases"] = [
    {
      id: "reset",
      source_span: sourceLineSpan(splash, 5, 24),
      operations: [
        {
          op: "stop_audio",
          audio: "MUSIC_NONE",
          source_span: sourceLineSpan(splash, 6, 7),
        },
        ...clearBgOperations,
        ...clearTileOperations,
        ...registerWrites,
        ...waitBgOperations,
        paletteLayoutOperation,
        {
          op: "set_default_palettes",
          dmg: { rBGP: 0xe4, rOBP0: 0xd0, rOBP1: 0xd0 },
          cgb: { background: 0xe4, objects: [0xe4, 0xe4] },
          source_span: sourceLineSpan(splash, 22),
          implementation_source_span: sourceLineSpan(tilemap, 146, 166),
        },
        waitOperation(10, sourceLineSpan(splash, 23, 24)),
      ],
    },
    {
      id: "copyright",
      source_span: sourceLineSpan(splash, 26, 34),
      operations: [
        ...clearTileOperations,
        ...resourceTransfers
          .filter((transfer) => String(transfer.id).startsWith("font_extra_"))
          .map((transfer) => ({
            op: "resource_transfer",
            transfer: transfer.id,
            source_span: transfer.source_span,
          })),
        {
          op: "resource_transfer",
          transfer: "copyright_tiles",
          source_span: copyrightSpan,
        },
        {
          op: "tilemap_write",
          write: "copyright_string",
          source_span: copyrightSpan,
        },
        ...waitBgOperations,
        waitOperation(100, sourceLineSpan(splash, 29, 30)),
        ...clearTileOperations,
      ],
    },
    {
      id: "dmg_nonreturn",
      source_span: sourceSpanThrough(gbcEntrySpan, gbcLoopSpan),
      operations: [
        {
          op: "stop_audio",
          audio: "MUSIC_NONE",
          condition: { source: "hCGB", predicate: "zero" },
          source_span: sourceLineSpan(gbcOnly, 6, 7),
        },
        ...clearTileOperations.map((operation) => ({
          ...operation,
          condition: { source: "hCGB", predicate: "zero" },
        })),
        ...resourceTransfers
          .filter((transfer) => String(transfer.id).startsWith("gbc_only"))
          .map((transfer) => ({
            op: "resource_transfer",
            transfer: transfer.id,
            condition: { source: "hCGB", predicate: "zero" },
            source_span: transfer.source_span,
          })),
        ...tilemapWrites
          .filter((write) => String(write.id).startsWith("gbc_only"))
          .map((write) => ({
            op: "tilemap_write",
            write: write.id,
            condition: { source: "hCGB", predicate: "zero" },
            source_span: write.source_span,
          })),
        ...waitBgOperations.map((operation) => ({
          ...operation,
          condition: { source: "hCGB", predicate: "zero" },
        })),
        {
          op: "repeat_wait_frames",
          frames: 1,
          condition: { source: "hCGB", predicate: "zero" },
          source_span: gbcLoopSpan,
        },
      ],
    },
    {
      id: "animation_setup",
      source_span: sourceSpanThrough(sourceLineSpan(splash, 37), initSpan),
      operations: [
        ...resourceTransfers
          .filter((transfer) =>
            [
              "gamefreak_logo_tiles",
              "ditto_decompress",
              "ditto_tiles_bank_0",
              "ditto_tiles_bank_1",
            ].includes(String(transfer.id)),
          )
          .map((transfer) => ({
            op: "resource_transfer",
            transfer: transfer.id,
            source_span: transfer.source_span,
          })),
        ...animationMemoryOperations,
      ],
    },
    {
      id: "teardown",
      source_span: teardownSpan,
      operations: [
        teardownSpriteMemoryOperation,
        ...clearTileOperations,
        clearOamOperation,
        waitOperation(16, sourceLineSpan(splash, 121, 122)),
      ],
    },
  ];

  const audio: RuntimePresentationCallableSubprogram["audio"] = [
    {
      id: "MUSIC_NONE",
      kind: "silence",
      source_span: sourceLineSpan(splash, 6, 7),
    },
    {
      id: "SFX_GAME_FREAK_PRESENTS",
      kind: "sound_effect",
      source_span: sourceLineSpan(splash, 157, 158),
    },
    {
      id: "SFX_DITTO_BOUNCE",
      kind: "sound_effect",
      source_span: sourceLineSpan(splash, 270, 271),
    },
    {
      id: "SFX_DITTO_POP_UP",
      kind: "sound_effect",
      source_span: sourceLineSpan(splash, 281, 282),
    },
    {
      id: "SFX_DITTO_TRANSFORM",
      kind: "sound_effect",
      source_span: sourceLineSpan(splash, 302, 303),
    },
  ];

  return {
    id: "splash_screen",
    source_entry: "SplashScreen",
    accepted_call_forms: ["callfar"],
    result: {
      name: "splash_outcome",
      storage: "carry",
      domain: [
        {
          id: "cancelled",
          value: 1,
          condition: {
            kind: "masked_input_nonzero",
            source: "hJoyLast",
            mask: { symbol: "PAD_BUTTONS", value: padButtons },
          },
          source_span: cancelSpan,
        },
        {
          id: "completed",
          value: 0,
          condition: {
            kind: "memory_bit_set",
            source: "wJumptableIndex",
            bit: { symbol: "JUMPTABLE_EXIT_F", value: exitBit },
          },
          source_span: completeSpan,
        },
        {
          id: "non_returning_dmg",
          value: null,
          condition: {
            kind: "memory_zero",
            source: "hCGB",
          },
          source_span: sourceSpanThrough(gbcEntrySpan, gbcLoopSpan),
        },
      ],
    },
    phases,
    loop: {
      source_span: loopSpan,
      order: [
        "sample_input",
        "cancel_if_buttons",
        "test_exit",
        "dispatch_scene",
        "sprite_scheduler_step",
        "wait_frame",
        "repeat",
      ],
      input: {
        routine: "JoyTextDelay",
        result: "hJoyLast",
        menu_guard: "hInMenu",
        menu_zero_source: "hJoyPressed",
        menu_nonzero_source: "hJoyDown",
        repeat_delay: "wTextDelayFrames",
        pressed_repeat_reset: 15,
        idle_repeat_restart: 5,
        mask: { symbol: "PAD_BUTTONS", value: padButtons },
        source_span: inputSpan,
        implementation_source_spans: [
          inputSpan,
          inputOkSpan,
          inputDelaySpan,
          inputRestartSpan,
        ],
      },
      scene_dispatch: {
        table: sceneTable.table,
        index: "wJumptableIndex",
        domain: sceneTable.index_domain,
        source_span: sceneTable.source_span,
        timer_effects: [
          {
            scene: 1,
            threshold: gameFreakWait,
            tilemap_write: "game_freak_text",
            audio: "SFX_GAME_FREAK_PRESENTS",
            dispatch_tick: gameFreakDispatchTick,
            source_spans: [sceneWaitSpan, sceneGameFreakSpan],
          },
          {
            scene: 2,
            threshold: presentsWait,
            tilemap_write: "presents_text",
            dispatch_tick: presentsDispatchTick,
            source_spans: [scenePresentsWaitSpan, scenePresentsSpan],
          },
          {
            scene: 3,
            threshold: finishWait,
            sets_exit_bit: true,
            dispatch_tick: naturalSchedulerTicks,
            source_spans: [sceneFinishSpan, sceneExitSpan],
          },
        ],
      },
      natural_scheduler_ticks: naturalSchedulerTicks,
      scheduler,
      frame_wait: frameWait,
    },
    resource_transfers: resourceTransfers,
    tilemap_writes: tilemapWrites,
    resources,
    audio,
    sprite_operations: splashSpriteOperations,
    sprite_programs: [splashProgram],
    required_consumer: {
      id: "runtime_title_screen.splash_screen",
      required: true,
    },
    source_span: sourceSpanThrough(splashSpan, completeSpan),
    implementation_source_spans: [
      splashSpan,
      loopSpan,
      cancelSpan,
      completeSpan,
      initSpan,
      teardownSpan,
      sceneWaitSpan,
      sceneGameFreakSpan,
      scenePresentsWaitSpan,
      scenePresentsSpan,
      sceneFinishSpan,
      sceneExitSpan,
      gbcEntrySpan,
      gbcLoopSpan,
      copyrightSpan,
      loadFontsWrapperSpan,
      loadFontsExtra1Span,
      loadFontsExtra2Span,
      loadFrameSpan,
      maskBitsSpan,
      frameTableSpan,
      layoutEntrySpan,
      cgbDispatchEntrySpan,
      cgbDispatchSpan,
      sgbDispatchEntrySpan,
      sgbDispatchSpan,
      sgbReturnSpan,
      cgbGameFreakSpan,
      sgbGameFreakSpan,
      sgbPalPacketSpan,
      sgbPalMacroSpan,
      sgbBlockPacketSpan,
      getPredefPaletteSpan,
      loadPaletteSpan,
      resetBgPaletteSpan,
      wipeAttrmapSpan,
      applyPalettesSpan,
      applyAttrmapEnabledSpan,
      applyAttrmapDisabledSpan,
      pushSgbPaletteSpan,
      inputSpan,
      inputOkSpan,
      inputDelaySpan,
      inputRestartSpan,
      clearOamSpan,
      { file: wram.file, start_line: 199, end_line: 308 },
    ],
  };
}

function certifyIntroClearBgPalettesOperations(
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationOperation[] {
  const intro = loadSource("engine/movie/intro.asm", options);
  const copy = loadSource("home/copy.asm", options);
  const delay = loadSource("home/delay.asm", options);
  const palettes = loadSource("home/palettes.asm", options);
  const wram = loadSource("ram/wram.asm", options);
  const hardware = loadSource("constants/hardware.inc", options);
  const blocks = parseAsmBlocks([intro, copy, delay, palettes]);
  requireExactRoutineBlock(
    blocks,
    "Intro_ClearBGPals",
    [
      "ldh a, [rWBK]",
      "push af",
      "ld a, BANK(wBGPals2)",
      "ldh [rWBK], a",
      "ld hl, wBGPals2",
      "ld bc, 16 palettes",
      "xor a",
      "call ByteFill",
      "pop af",
      "ldh [rWBK], a",
      "ld a, TRUE",
      "ldh [hCGBPalUpdate], a",
      "call DelayFrame",
      "call DelayFrame",
      "ret",
    ],
    "Intro_ClearBGPals exact 16-palette clear, request, and two-VBlank wait",
  );
  requireExactRoutineBlock(
    blocks,
    "ByteFill",
    ["inc b", "inc c", "jr .HandleLoop"],
    "Intro_ClearBGPals ByteFill entry",
  );
  requireExactRoutineBlock(
    blocks,
    ".PutByte@ByteFill",
    ["ld [hli], a"],
    "Intro_ClearBGPals ByteFill byte write",
  );
  requireExactRoutineBlock(
    blocks,
    ".HandleLoop@ByteFill",
    ["dec c", "jr nz, .PutByte", "dec b", "jr nz, .PutByte", "ret"],
    "Intro_ClearBGPals ByteFill exact count loop",
  );
  const cgbGateSpan = requireExactRoutineBlock(
    blocks,
    "UpdatePalsIfCGB",
    ["ldh a, [hCGB]", "and a", "ret z"],
    "Intro_ClearBGPals VBlank CGB-only palette consumer gate",
  );
  const delayFramesImplementationSpan = certifyDelayFrames(blocks);

  const constants = parseAsmConstants([hardware]);
  if (constants.get("PAL_SIZE") !== 8) {
    throw new Error(
      "Intro_ClearBGPals requires exact eight-byte source palettes",
    );
  }
  const paletteBufferSize = asmRegionByteSize(
    wram,
    "wBGPals2:: ds 8 palettes",
    "wOBPals2:: ds 8 palettes",
    { constants, unitExpansions: { palettes: "* PAL_SIZE" } },
  );
  if (paletteBufferSize !== 128) {
    throw new Error(
      `Intro_ClearBGPals must clear exactly 16 palettes/128 bytes; reached ${paletteBufferSize}`,
    );
  }

  const sourceSpan = (
    startIndex: number,
    endIndex: number = startIndex,
  ): RuntimePresentationSourceSpan => {
    const block = blocks.get("Intro_ClearBGPals");
    const start = block?.instructions[startIndex]?.source_span;
    const end = block?.instructions[endIndex]?.source_span;
    if (!start || !end) {
      throw new Error(
        `Intro_ClearBGPals cannot resolve source instructions ${startIndex}..${endIndex}`,
      );
    }
    return sourceSpanThrough(start, end);
  };
  const sharedPaletteTransfer = certifyClearBgPalettesOperations(options).find(
    (operation) => operation.op === "palette_transfer_request",
  );
  if (!sharedPaletteTransfer) {
    throw new Error(
      "Intro_ClearBGPals has no exact CGB VBlank palette-transfer implementation",
    );
  }
  const always = { source: null, predicate: "always", source_span: null };
  return [
    {
      op: "fill_memory",
      target: "wBGPals2",
      byte_count: paletteBufferSize,
      value: 0,
      direction: "ascending",
      bank: { select: "BANK(wBGPals2)", restore: true },
      condition: always,
      source_span: sourceSpan(0, 9),
    },
    {
      op: "write_memory_byte",
      target: "hCGBPalUpdate",
      value: 1,
      address_space: "hram",
      condition: always,
      source_span: sourceSpan(10, 11),
    },
    {
      ...sharedPaletteTransfer,
      condition: {
        source: "hCGB",
        predicate: "nonzero",
        source_span: cgbGateSpan,
      },
      source_span: sourceSpan(10, 12),
    },
    {
      op: "wait_frames",
      frames: 1,
      condition: always,
      source_span: sourceSpan(12),
    },
    {
      op: "wait_frames",
      frames: 1,
      condition: always,
      source_span: sourceSpan(13),
    },
  ];
}

type RuntimePresentationScratchSegment = {
  resource: string;
  resource_offset: number;
  scratch_offset: number;
  byte_count: number;
};

type RuntimePresentationDecompressionImplementation = {
  algorithm_source_span: RuntimePresentationSourceSpan;
  helper_source_spans: Map<
    string,
    { source_span: RuntimePresentationSourceSpan; tile_count: number }
  >;
  request_source_spans: RuntimePresentationSourceSpan[];
  service_source_spans: RuntimePresentationSourceSpan[];
  vblank_source_span: RuntimePresentationSourceSpan;
  scratch_declaration_source_span: RuntimePresentationSourceSpan;
  request_state_source_spans: RuntimePresentationSourceSpan[];
  tile_size_source_span: RuntimePresentationSourceSpan;
  coordinate_macro_source_span: RuntimePresentationSourceSpan;
  tile_macro_source_span: RuntimePresentationSourceSpan;
  default_tiles_per_cycle: number;
  mobile_tiles_per_cycle: number;
  scratch_capacity_bytes: number;
  bytes_per_tile: number;
};

const requireNormalizedSourceSequence = (
  source: LoadedSource,
  expected: readonly string[],
  context: string,
): RuntimePresentationSourceSpan => {
  const lines = source.lines
    .map((line, index) => ({ line: normalizeAsmLine(line), index }))
    .filter(({ line }) => line.length > 0);
  const start = lines.findIndex((_, candidate) =>
    expected.every(
      (value, offset) => lines[candidate + offset]?.line === value,
    ),
  );
  if (start < 0) {
    throw new Error(
      `${context} expected exact source sequence ${expected.join(" -> ")} in ${source.file}`,
    );
  }
  return {
    file: source.file,
    start_line: lines[start].index + 1,
    end_line: lines[start + expected.length - 1].index + 1,
  };
};

const certifyPresentationLz3Implementation = (
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationSourceSpan => {
  const source = loadSource("home/decompress.asm", options);
  const blocks = parseAsmBlocks([source]);
  for (const constant of [
    "DEF LZ_END EQU $ff",
    "DEF LZ_CMD EQU %11100000",
    "DEF LZ_LEN EQU %00011111",
    "DEF LZ_LITERAL EQU 0 << 5",
    "DEF LZ_ITERATE EQU 1 << 5",
    "DEF LZ_ALTERNATE EQU 2 << 5",
    "DEF LZ_ZERO EQU 3 << 5",
    "DEF LZ_RW EQU 2 + 5",
    "DEF LZ_REPEAT EQU 4 << 5",
    "DEF LZ_FLIP EQU 5 << 5",
    "DEF LZ_REVERSE EQU 6 << 5",
    "DEF LZ_LONG EQU 7 << 5",
    "DEF LZ_LONG_HI EQU %00000011",
  ]) {
    requireExactNormalizedLine(source, constant, "LZ3 command definition");
  }
  const spans = [
    requireInstructionSubsequence(
      blocks,
      "Decompress",
      ["ld a, e", "ld [wLZAddress], a", "ld a, d", "ld [wLZAddress + 1], a"],
      "LZ3 output-base initialization",
    ),
    requireExactRoutineBlock(
      blocks,
      ".Main@Decompress",
      [
        "ld a, [hl]",
        "cp LZ_END",
        "ret z",
        "and LZ_CMD",
        "cp LZ_LONG",
        "jr nz, .short",
        "ld a, [hl]",
        "add a",
        "add a",
        "add a",
        "and LZ_CMD",
        "push af",
        "ld a, [hli]",
        "and LZ_LONG_HI",
        "ld b, a",
        "ld a, [hli]",
        "ld c, a",
        "inc bc",
        "jr .command",
      ],
      "LZ3 terminator and long-command decode",
    ),
    requireExactRoutineBlock(
      blocks,
      ".short@Decompress",
      ["push af", "ld a, [hli]", "and LZ_LEN", "ld c, a", "ld b, 0", "inc c"],
      "LZ3 short-command decode",
    ),
    requireExactRoutineBlock(
      blocks,
      ".command@Decompress",
      [
        "inc b",
        "inc c",
        "pop af",
        "bit LZ_RW, a",
        "jr nz, .rewrite",
        "cp LZ_ITERATE",
        "jr z, .Iter",
        "cp LZ_ALTERNATE",
        "jr z, .Alt",
        "cp LZ_ZERO",
        "jr z, .Zero",
      ],
      "LZ3 command dispatch",
    ),
    requireExactRoutineBlock(
      blocks,
      ".lloop@Decompress",
      ["dec c", "jr nz, .lnext", "dec b", "jp z, .Main"],
      "LZ3 literal length loop",
    ),
    requireExactRoutineBlock(
      blocks,
      ".lnext@Decompress",
      ["ld a, [hli]", "ld [de], a", "inc de", "jr .lloop"],
      "LZ3 literal copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".Iter@Decompress",
      ["ld a, [hli]"],
      "LZ3 iterate source",
    ),
    requireExactRoutineBlock(
      blocks,
      ".iloop@Decompress",
      ["dec c", "jr nz, .inext", "dec b", "jp z, .Main"],
      "LZ3 iterate length loop",
    ),
    requireExactRoutineBlock(
      blocks,
      ".inext@Decompress",
      ["ld [de], a", "inc de", "jr .iloop"],
      "LZ3 iterate copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".Alt@Decompress",
      ["dec c", "jr nz, .anext1", "dec b", "jp z, .adone1"],
      "LZ3 alternate first-byte length loop",
    ),
    requireExactRoutineBlock(
      blocks,
      ".anext1@Decompress",
      [
        "ld a, [hli]",
        "ld [de], a",
        "inc de",
        "dec c",
        "jr nz, .anext2",
        "dec b",
        "jp z, .adone2",
      ],
      "LZ3 alternate first-byte copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".anext2@Decompress",
      ["ld a, [hld]", "ld [de], a", "inc de", "jr .Alt"],
      "LZ3 alternate second-byte copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".adone1@Decompress",
      ["inc hl"],
      "LZ3 alternate first-byte completion",
    ),
    requireExactRoutineBlock(
      blocks,
      ".adone2@Decompress",
      ["inc hl", "jr .Main"],
      "LZ3 alternate completion",
    ),
    requireExactRoutineBlock(
      blocks,
      ".Zero@Decompress",
      ["xor a"],
      "LZ3 zero value",
    ),
    requireExactRoutineBlock(
      blocks,
      ".zloop@Decompress",
      ["dec c", "jr nz, .znext", "dec b", "jp z, .Main"],
      "LZ3 zero length loop",
    ),
    requireExactRoutineBlock(
      blocks,
      ".znext@Decompress",
      ["ld [de], a", "inc de", "jr .zloop"],
      "LZ3 zero copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".rewrite@Decompress",
      [
        "push hl",
        "push af",
        "ld a, [hli]",
        "bit 7, a",
        "jr z, .positive",
        "and %01111111",
        "cpl",
        "add e",
        "ld l, a",
        "ld a, -1",
        "adc d",
        "ld h, a",
        "jr .ok",
      ],
      "LZ3 negative rewrite offset",
    ),
    requireExactRoutineBlock(
      blocks,
      ".positive@Decompress",
      [
        "ld l, [hl]",
        "ld h, a",
        "ld a, [wLZAddress]",
        "add l",
        "ld l, a",
        "ld a, [wLZAddress + 1]",
        "adc h",
        "ld h, a",
      ],
      "LZ3 positive rewrite offset",
    ),
    requireExactRoutineBlock(
      blocks,
      ".ok@Decompress",
      [
        "pop af",
        "cp LZ_REPEAT",
        "jr z, .Repeat",
        "cp LZ_FLIP",
        "jr z, .Flip",
        "cp LZ_REVERSE",
        "jr z, .Reverse",
      ],
      "LZ3 rewrite dispatch",
    ),
    requireExactRoutineBlock(
      blocks,
      ".Repeat@Decompress",
      ["dec c", "jr nz, .rnext", "dec b", "jr z, .donerw"],
      "LZ3 repeat length loop",
    ),
    requireExactRoutineBlock(
      blocks,
      ".rnext@Decompress",
      ["ld a, [hli]", "ld [de], a", "inc de", "jr .Repeat"],
      "LZ3 repeat copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".Flip@Decompress",
      ["dec c", "jr nz, .fnext", "dec b", "jp z, .donerw"],
      "LZ3 flip length loop",
    ),
    requireExactRoutineBlock(
      blocks,
      ".fnext@Decompress",
      ["ld a, [hli]", "push bc", "lb bc, 0, 8"],
      "LZ3 flip source",
    ),
    requireExactRoutineBlock(
      blocks,
      ".floop@Decompress",
      [
        "rra",
        "rl b",
        "dec c",
        "jr nz, .floop",
        "ld a, b",
        "pop bc",
        "ld [de], a",
        "inc de",
        "jr .Flip",
      ],
      "LZ3 bit-flip copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".Reverse@Decompress",
      ["dec c", "jr nz, .rvnext", "dec b", "jp z, .donerw"],
      "LZ3 reverse length loop",
    ),
    requireExactRoutineBlock(
      blocks,
      ".rvnext@Decompress",
      ["ld a, [hld]", "ld [de], a", "inc de", "jr .Reverse"],
      "LZ3 reverse copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".donerw@Decompress",
      ["pop hl", "bit 7, [hl]", "jr nz, .next", "inc hl"],
      "LZ3 rewrite operand completion",
    ),
    requireExactRoutineBlock(
      blocks,
      ".next@Decompress",
      ["inc hl", "jp .Main"],
      "LZ3 command continuation",
    ),
  ];
  void spans;
  return findLabelSpan(source, "Decompress");
};

const decodePresentationLz3Resource = (
  relativePath: string,
  options: BuildRuntimeTitlePresentationProgramOptions,
): { compressed_byte_count: number; output_byte_count: number } => {
  const root = path.resolve(options.disassemblyRoot);
  const absolute = path.resolve(root, relativePath);
  if (absolute !== root && !absolute.startsWith(`${root}${path.sep}`)) {
    throw new Error(
      `Runtime presentation compressed resource ${relativePath} escapes the disassembly root`,
    );
  }
  const input = fs.readFileSync(absolute);
  const output: number[] = [];
  let cursor = 0;
  let terminated = false;
  const readByte = (): number => {
    const value = input[cursor];
    if (value === undefined) {
      throw new Error(
        `Runtime presentation LZ3 resource ${relativePath} ended before its $ff terminator`,
      );
    }
    cursor += 1;
    return value;
  };
  while (cursor < input.length) {
    const control = readByte();
    if (control === 0xff) {
      terminated = true;
      break;
    }
    let command = control & 0xe0;
    let length: number;
    if (command === 0xe0) {
      command = (control & 0x1c) << 3;
      length = (((control & 0x03) << 8) | readByte()) + 1;
    } else {
      length = (control & 0x1f) + 1;
    }
    if (command < 0x80) {
      if (command === 0x00) {
        for (let index = 0; index < length; index += 1) {
          output.push(readByte());
        }
      } else if (command === 0x20) {
        const value = readByte();
        output.push(...Array.from({ length }, () => value));
      } else if (command === 0x40) {
        const first = readByte();
        const second = readByte();
        output.push(
          ...Array.from({ length }, (_, index) =>
            index % 2 === 0 ? first : second,
          ),
        );
      } else if (command === 0x60) {
        output.push(...Array.from({ length }, () => 0));
      } else {
        throw new Error(
          `Runtime presentation LZ3 resource ${relativePath} reached unsupported command ${command}`,
        );
      }
    } else {
      const offsetHigh = readByte();
      let sourceIndex: number;
      if ((offsetHigh & 0x80) !== 0) {
        sourceIndex = output.length - (offsetHigh & 0x7f) - 1;
      } else {
        sourceIndex = ((offsetHigh & 0x7f) << 8) | readByte();
      }
      for (let index = 0; index < length; index += 1) {
        const value = output[sourceIndex];
        if (value === undefined) {
          throw new Error(
            `Runtime presentation LZ3 resource ${relativePath} rewrites outside its ${output.length}-byte output`,
          );
        }
        const flipped = Number.parseInt(
          value.toString(2).padStart(8, "0").split("").reverse().join(""),
          2,
        );
        output.push(command === 0xa0 ? flipped : value);
        sourceIndex += command === 0xc0 ? -1 : 1;
      }
    }
  }
  if (!terminated) {
    throw new Error(
      `Runtime presentation LZ3 resource ${relativePath} has no $ff terminator`,
    );
  }
  return {
    compressed_byte_count: input.length,
    output_byte_count: output.length,
  };
};

const requirePresentationResourceAtLabel = (
  source: LoadedSource,
  symbol: string,
): {
  path: string;
  kind: "tiles" | "tilemap" | "attrmap" | "palette";
  label_source_span: RuntimePresentationSourceSpan;
  directive_source_span: RuntimePresentationSourceSpan;
} => {
  const labels = source.lines.flatMap((line, index) =>
    labelPattern(symbol).test(normalizeAsmLine(line)) ? [index] : [],
  );
  if (labels.length !== 1) {
    throw new Error(
      `Runtime presentation resource ${symbol} must have one exact source label in ${source.file}`,
    );
  }
  const labelIndex = labels[0];
  let directiveIndex = labelIndex + 1;
  while (
    directiveIndex < source.lines.length &&
    normalizeAsmLine(source.lines[directiveIndex]).length === 0
  ) {
    directiveIndex += 1;
  }
  const directive = normalizeAsmLine(source.lines[directiveIndex] ?? "");
  const match = directive.match(/^INCBIN\s+"([^"]+)"$/);
  if (!match) {
    throw new Error(
      `Runtime presentation resource ${symbol} has no exact unmodified INCBIN payload after ${source.file}:${labelIndex + 1}`,
    );
  }
  const span = {
    file: source.file,
    start_line: directiveIndex + 1,
    end_line: directiveIndex + 1,
  };
  return {
    path: match[1],
    kind: resourceKind(match[1]),
    label_source_span: {
      file: source.file,
      start_line: labelIndex + 1,
      end_line: labelIndex + 1,
    },
    directive_source_span: span,
  };
};

const certifyIntroDecompressionImplementation = (
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationDecompressionImplementation => {
  const intro = loadSource("engine/movie/intro.asm", options);
  const gfx = loadSource("home/gfx.asm", options);
  const video = loadSource("home/video.asm", options);
  const vblank = loadSource("home/vblank.asm", options);
  const wram = loadSource("ram/wram.asm", options);
  const hram = loadSource("ram/hram.asm", options);
  const vram = loadSource("ram/vram.asm", options);
  const hardware = loadSource("constants/hardware.inc", options);
  const codeMacros = loadSource("macros/code.asm", options);
  const gfxMacros = loadSource("macros/gfx.asm", options);
  const coordMacros = loadSource("macros/coords.asm", options);
  const blocks = parseAsmBlocks([intro, gfx, video, vblank]);

  const helperSourceSpans = new Map<
    string,
    { source_span: RuntimePresentationSourceSpan; tile_count: number }
  >();
  for (const [helper, block] of blocks) {
    const staticShape = block.instructions.map((instruction, index) =>
      index === 9
        ? "<packed request count>"
        : instructionSignature(instruction),
    );
    if (
      staticShape.join("\0") !==
      [
        "ldh a, [rWBK]",
        "push af",
        "ld a, BANK(wDecompressScratch)",
        "ldh [rWBK], a",
        "push de",
        "ld de, wDecompressScratch",
        "call Decompress",
        "pop hl",
        "ld de, wDecompressScratch",
        "<packed request count>",
        "call Request2bpp",
        "pop af",
        "ldh [rWBK], a",
        "ret",
      ].join("\0")
    ) {
      continue;
    }
    const countInstruction = block?.instructions[9];
    const countOperand = countInstruction?.args[2];
    if (
      countInstruction?.opcode !== "lb" ||
      countInstruction.args[0] !== "bc" ||
      countInstruction.args[1] !== "$01" ||
      !countOperand
    ) {
      throw new Error(
        `${helper} has no exact source-derived $01 bank and tile-count operand`,
      );
    }
    const tileCount = evaluateAsmInteger(countOperand, new Map());
    const span = requireExactRoutineBlock(
      blocks,
      helper,
      [
        "ldh a, [rWBK]",
        "push af",
        "ld a, BANK(wDecompressScratch)",
        "ldh [rWBK], a",
        "push de",
        "ld de, wDecompressScratch",
        "call Decompress",
        "pop hl",
        "ld de, wDecompressScratch",
        `lb bc, $01, ${countOperand}`,
        "call Request2bpp",
        "pop af",
        "ldh [rWBK], a",
        "ret",
      ],
      `${helper} exact decompression/request bridge`,
    );
    helperSourceSpans.set(helper, { source_span: span, tile_count: tileCount });
  }
  if (helperSourceSpans.size === 0) {
    throw new Error(
      "Intro source has no structurally certified decompression/request helper",
    );
  }

  const defaultTilesSpan = requireExactNormalizedLine(
    gfx,
    "DEF TILES_PER_CYCLE EQU 8",
    "Request2bpp default VBlank chunk",
  );
  const mobileTilesSpan = requireExactNormalizedLine(
    gfx,
    "DEF MOBILE_TILES_PER_CYCLE EQU 6",
    "Request2bpp mobile VBlank chunk",
  );
  const requestSourceSpans = [
    requireExactRoutineBlock(
      blocks,
      "Request2bpp",
      [
        "ldh a, [hBGMapMode]",
        "push af",
        "xor a",
        "ldh [hBGMapMode], a",
        "ldh a, [hROMBank]",
        "push af",
        "ld a, b",
        "rst Bankswitch",
        "ldh a, [hTilesPerCycle]",
        "push af",
        "ld a, TILES_PER_CYCLE",
        "ldh [hTilesPerCycle], a",
        "ld a, [wLinkMode]",
        "cp LINK_MOBILE",
        "jr nz, .NotMobile",
        "ldh a, [hMobile]",
        "and a",
        "jr nz, .NotMobile",
        "ld a, MOBILE_TILES_PER_CYCLE",
        "ldh [hTilesPerCycle], a",
      ],
      "Request2bpp exact saved-state and chunk selection",
    ),
    requireExactRoutineBlock(
      blocks,
      ".NotMobile@Request2bpp",
      [
        "ld a, e",
        "ld [wRequested2bppSource], a",
        "ld a, d",
        "ld [wRequested2bppSource + 1], a",
        "ld a, l",
        "ld [wRequested2bppDest], a",
        "ld a, h",
        "ld [wRequested2bppDest + 1], a",
      ],
      "Request2bpp exact request pointer writes",
    ),
    requireExactRoutineBlock(
      blocks,
      ".loop@Request2bpp",
      [
        "ld a, c",
        "ld hl, hTilesPerCycle",
        "cp [hl]",
        "jr nc, .cycle",
        "ld [wRequested2bppSize], a",
      ],
      "Request2bpp final chunk queue",
    ),
    requireExactRoutineBlock(
      blocks,
      ".wait@Request2bpp",
      [
        "call DelayFrame",
        "ld a, [wRequested2bppSize]",
        "and a",
        "jr nz, .wait",
        "pop af",
        "ldh [hTilesPerCycle], a",
        "pop af",
        "rst Bankswitch",
        "pop af",
        "ldh [hBGMapMode], a",
        "ret",
      ],
      "Request2bpp blocking completion and saved-state restoration",
    ),
    requireExactRoutineBlock(
      blocks,
      ".cycle@Request2bpp",
      ["ldh a, [hTilesPerCycle]", "ld [wRequested2bppSize], a"],
      "Request2bpp full chunk queue",
    ),
    requireExactRoutineBlock(
      blocks,
      ".wait2@Request2bpp",
      [
        "call DelayFrame",
        "ld a, [wRequested2bppSize]",
        "and a",
        "jr nz, .wait2",
        "ld a, c",
        "ld hl, hTilesPerCycle",
        "sub [hl]",
        "ld c, a",
        "jr .loop",
      ],
      "Request2bpp full-chunk completion and remainder",
    ),
    defaultTilesSpan,
    mobileTilesSpan,
  ];

  const serviceSourceSpans = [
    requireExactRoutineBlock(
      blocks,
      "Serve2bppRequest",
      [
        "ld a, [wRequested2bppSize]",
        "and a",
        "ret z",
        "ldh a, [rLY]",
        "cp LY_VBLANK",
        "ret c",
        "cp LY_VBLANK + 2",
        "ret nc",
        "jr _Serve2bppRequest",
      ],
      "Serve2bppRequest exact first-fifth VBlank guard",
    ),
    requireExactRoutineBlock(
      blocks,
      "Serve2bppRequest_VBlank",
      ["ld a, [wRequested2bppSize]", "and a", "ret z"],
      "Serve2bppRequest exact unguarded VBlank entry",
    ),
    requireExactRoutineBlock(
      blocks,
      "_Serve2bppRequest",
      [
        "ld [hSPBuffer], sp",
        "ld hl, wRequested2bppSource",
        "ld a, [hli]",
        "ld h, [hl]",
        "ld l, a",
        "ld sp, hl",
        "ld hl, wRequested2bppDest",
        "ld a, [hli]",
        "ld h, [hl]",
        "ld l, a",
        "ld a, [wRequested2bppSize]",
        "ld b, a",
        "xor a",
        "ld [wRequested2bppSize], a",
      ],
      "Serve2bppRequest exact request capture and pre-copy completion clear",
    ),
    requireExactRoutineBlock(
      blocks,
      ".next@_Serve2bppRequest",
      [
        "rept 7",
        "pop de",
        "ld [hl], e",
        "inc l",
        "ld [hl], d",
        "inc l",
        "endr",
        "pop de",
        "ld [hl], e",
        "inc l",
        "ld [hl], d",
        "inc hl",
        "dec b",
        "jr nz, .next",
        "ld a, l",
        "ld [wRequested2bppDest], a",
        "ld a, h",
        "ld [wRequested2bppDest + 1], a",
        "ld [wRequested2bppSource], sp",
        "ldh a, [hSPBuffer]",
        "ld l, a",
        "ldh a, [hSPBuffer + 1]",
        "ld h, a",
        "ld sp, hl",
        "ret",
      ],
      "Serve2bppRequest exact 16-byte tile copy and pointer advancement",
    ),
  ];
  const vblankSourceSpan = requireInstructionSubsequence(
    blocks,
    "VBlank_Normal",
    [
      "call UpdateBGMapBuffer",
      "jr c, .done",
      "call UpdatePalsIfCGB",
      "jr c, .done",
      "call DMATransfer",
      "jr c, .done",
      "call UpdateBGMap",
      "call Serve2bppRequest",
      "call Serve1bppRequest",
      "call AnimateTileset",
    ],
    "Serve2bppRequest exact VBlank priority order",
  );

  const scratchDeclarationSourceSpan = requireExactNormalizedLine(
    wram,
    "wDecompressScratch:: ds $80 tiles",
    "Intro decompression scratch capacity",
  );
  const requestStateSourceSpans = [
    requireExactNormalizedLine(
      wram,
      "wRequested2bppSize:: db",
      "Request2bpp size state",
    ),
    requireExactNormalizedLine(
      wram,
      "wRequested2bppSource:: dw",
      "Request2bpp source state",
    ),
    requireExactNormalizedLine(
      wram,
      "wRequested2bppDest:: dw",
      "Request2bpp destination state",
    ),
    requireExactNormalizedLine(
      hram,
      "hTilesPerCycle:: db",
      "Request2bpp chunk-size state",
    ),
    requireExactNormalizedLine(
      hram,
      "hBGMapMode:: db",
      "Request2bpp BG-map mode state",
    ),
  ];
  requireExactNormalizedLine(
    vram,
    "vBGMap0:: ds TILEMAP_AREA",
    "Intro decompression BG-map destination",
  );
  for (const target of ["vTiles0", "vTiles1", "vTiles2"]) {
    requireExactNormalizedLine(
      vram,
      `${target}:: ds $80 tiles`,
      `Intro decompression ${target} destination`,
    );
  }
  requireExactNormalizedLine(
    hardware,
    "def rVBK equ $FF4F",
    "Intro decompression VRAM bank register",
  );
  requireExactNormalizedLine(
    hardware,
    "def rWBK equ $FF70",
    "Intro decompression WRAM bank register",
  );
  const tileSizeSourceSpan = requireExactNormalizedLine(
    hardware,
    "def TILE_SIZE equ 16",
    "Request2bpp exact 2bpp tile size",
  );
  const tileMacroSourceSpan = requireExactNormalizedLine(
    gfxMacros,
    'DEF tile EQUS "+ TILE_SIZE *"',
    "Intro decompression tile-address expression",
  );
  requireNormalizedSourceSequence(
    codeMacros,
    ["MACRO? lb", "ld \\1, ((\\2) & $ff) << 8 | ((\\3) & $ff)", "ENDM"],
    "Intro decompression exact lb register packing",
  );
  const coordinateMacroSourceSpan = requireNormalizedSourceSequence(
    coordMacros,
    [
      "MACRO? debgcoord",
      "bgcoord de, \\#",
      "ENDM",
      "MACRO? bgcoord",
      "if _NARG < 4",
      "ld \\1, (\\3) * TILEMAP_WIDTH + (\\2) + vBGMap0",
      "else",
      "ld \\1, (\\3) * TILEMAP_WIDTH + (\\2) + \\4",
      "endc",
      "ENDM",
    ],
    "Intro decompression exact BG-map coordinate expansion",
  );

  return {
    algorithm_source_span: certifyPresentationLz3Implementation(options),
    helper_source_spans: helperSourceSpans,
    request_source_spans: requestSourceSpans,
    service_source_spans: serviceSourceSpans,
    vblank_source_span: vblankSourceSpan,
    scratch_declaration_source_span: scratchDeclarationSourceSpan,
    request_state_source_spans: requestStateSourceSpans,
    tile_size_source_span: tileSizeSourceSpan,
    coordinate_macro_source_span: coordinateMacroSourceSpan,
    tile_macro_source_span: tileMacroSourceSpan,
    default_tiles_per_cycle: 8,
    mobile_tiles_per_cycle: 6,
    scratch_capacity_bytes: 128 * 16,
    bytes_per_tile: 16,
  };
};

const overlayPresentationScratchPrefix = (
  existing: readonly RuntimePresentationScratchSegment[],
  resource: string,
  outputByteCount: number,
): RuntimePresentationScratchSegment[] =>
  [
    {
      resource,
      resource_offset: 0,
      scratch_offset: 0,
      byte_count: outputByteCount,
    },
    ...existing.flatMap((segment) => {
      const end = segment.scratch_offset + segment.byte_count;
      if (end <= outputByteCount) return [];
      const start = Math.max(segment.scratch_offset, outputByteCount);
      return [
        {
          resource: segment.resource,
          resource_offset:
            segment.resource_offset + (start - segment.scratch_offset),
          scratch_offset: start,
          byte_count: end - start,
        },
      ];
    }),
  ].sort((left, right) => left.scratch_offset - right.scratch_offset);

const requirePresentationScratchSegments = (
  segments: readonly RuntimePresentationScratchSegment[],
  byteCount: number,
  context: string,
): RuntimePresentationScratchSegment[] => {
  const result: RuntimePresentationScratchSegment[] = [];
  let cursor = 0;
  for (const segment of segments) {
    if (cursor >= byteCount) break;
    if (segment.scratch_offset !== cursor) {
      throw new Error(
        `${context} reads uninitialized decompression scratch at byte ${cursor}`,
      );
    }
    const count = Math.min(segment.byte_count, byteCount - cursor);
    result.push({ ...segment, byte_count: count });
    cursor += count;
  }
  if (cursor !== byteCount) {
    throw new Error(
      `${context} reads uninitialized decompression scratch at byte ${cursor} of ${byteCount}`,
    );
  }
  return result;
};

const resolveIntroTransferDestination = (
  instruction: RuntimePresentationAsmInstruction,
  options: BuildRuntimeTitlePresentationProgramOptions,
  implementation: RuntimePresentationDecompressionImplementation,
): {
  target: string;
  target_byte_offset: number;
  source_span: RuntimePresentationSourceSpan;
  macro_source_span: RuntimePresentationSourceSpan;
} | null => {
  if (instruction.opcode === "debgcoord" && instruction.args.length === 2) {
    const x = evaluateAsmInteger(instruction.args[0], new Map());
    const y = evaluateAsmInteger(instruction.args[1], new Map());
    if (x < 0 || x >= 32 || y < 0 || y >= 32) {
      throw new Error(
        `IntroScene1 BG-map coordinate ${x},${y} is outside the source-proven 32x32 map`,
      );
    }
    const offset = y * 32 + x;
    return {
      target: offset === 0 ? "vBGMap0" : `vBGMap0 + ${offset}`,
      target_byte_offset: offset,
      source_span: instruction.source_span,
      macro_source_span: implementation.coordinate_macro_source_span,
    };
  }
  if (
    instruction.opcode === "ld" &&
    instruction.args[0] === "de" &&
    instruction.args[1]
  ) {
    const match = instruction.args[1].match(/^(vTiles[012])\s+tile\s+(.+)$/);
    if (!match) return null;
    const vram = loadSource("ram/vram.asm", options);
    if (!findAsmSymbolDeclarationSpan(match[1], [vram])) {
      throw new Error(
        `IntroScene1 transfer destination ${match[1]} has no exact VRAM declaration`,
      );
    }
    const tile = evaluateAsmInteger(match[2], new Map());
    if (tile < 0 || tile >= 128) {
      throw new Error(
        `IntroScene1 transfer destination tile ${tile} is outside its 128-tile VRAM region`,
      );
    }
    return {
      target: instruction.args[1],
      target_byte_offset: tile * implementation.bytes_per_tile,
      source_span: instruction.source_span,
      macro_source_span: implementation.tile_macro_source_span,
    };
  }
  return null;
};

const compileIntroSceneDecompressionPrefix = (
  firstScene: readonly RuntimePresentationAsmInstruction[],
  startIndex: number,
  initialVramBank: number,
  intro: LoadedSource,
  options: BuildRuntimeTitlePresentationProgramOptions,
): {
  operations: RuntimePresentationOperation[];
  consumed: number;
  vram_bank: number;
} => {
  const implementation = certifyIntroDecompressionImplementation(options);
  const operations: RuntimePresentationOperation[] = [];
  let scratch: RuntimePresentationScratchSegment[] = [];
  let resource: ReturnType<typeof requirePresentationResourceAtLabel> | null =
    null;
  let resourceSymbol: string | null = null;
  let resourceLoadSpan: RuntimePresentationSourceSpan | null = null;
  let destination: ReturnType<typeof resolveIntroTransferDestination> = null;
  let vramBank = initialVramBank;
  let index = startIndex;

  while (index < firstScene.length) {
    const instruction = firstScene[index];
    if (
      instruction.opcode === "ld" &&
      instruction.args[0] === "hl" &&
      instruction.args[1]
    ) {
      resourceSymbol = instruction.args[1];
      resource = requirePresentationResourceAtLabel(intro, resourceSymbol);
      resourceLoadSpan = instruction.source_span;
      index += 1;
      continue;
    }
    const resolvedDestination = resolveIntroTransferDestination(
      instruction,
      options,
      implementation,
    );
    if (resolvedDestination) {
      destination = resolvedDestination;
      index += 1;
      continue;
    }
    const bankRun = compileAccumulatorHighMemoryWriteRun(
      firstScene,
      index,
      options,
    );
    if (
      bankRun?.consumed === 2 &&
      bankRun.operations[0]?.target === "rVBK" &&
      typeof bankRun.operations[0].value === "number"
    ) {
      operations.push(...bankRun.operations);
      vramBank = bankRun.operations[0].value as number;
      index += bankRun.consumed;
      continue;
    }
    if (instruction.opcode === "call" && instruction.args.length === 1) {
      const helper = implementation.helper_source_spans.get(
        instruction.args[0],
      );
      if (!helper) break;
      if (!resource || !resourceSymbol || !resourceLoadSpan) {
        throw new Error(
          `IntroScene1 ${instruction.args[0]} has no source-derived compressed resource operand`,
        );
      }
      if (!destination) {
        throw new Error(
          `IntroScene1 ${instruction.args[0]} has no source-derived VRAM destination operand`,
        );
      }
      const decoded = decodePresentationLz3Resource(resource.path, options);
      if (decoded.output_byte_count > implementation.scratch_capacity_bytes) {
        throw new Error(
          `IntroScene1 resource ${resource.path} expands to ${decoded.output_byte_count} bytes beyond the ${implementation.scratch_capacity_bytes}-byte scratch buffer`,
        );
      }
      scratch = overlayPresentationScratchPrefix(
        scratch,
        resource.path,
        decoded.output_byte_count,
      );
      const transferByteCount =
        helper.tile_count * implementation.bytes_per_tile;
      const sourceSegments = requirePresentationScratchSegments(
        scratch,
        transferByteCount,
        `IntroScene1 ${resourceSymbol} request`,
      );
      const callSourceSpan = sourceSpanThrough(
        resourceLoadSpan,
        instruction.source_span,
      );
      operations.push(
        {
          op: "decompress_lz3_resource",
          resource: resource.path,
          resource_symbol: resourceSymbol,
          resource_kind: resource.kind,
          compressed_byte_count: decoded.compressed_byte_count,
          output_byte_count: decoded.output_byte_count,
          target: "wDecompressScratch",
          target_offset: 0,
          target_capacity_bytes: implementation.scratch_capacity_bytes,
          overwrites: "output_prefix_only",
          wram_bank: {
            register: "rWBK",
            select: "BANK(wDecompressScratch)",
            restore: true,
          },
          resource_label_source_span: resource.label_source_span,
          resource_source_span: resource.directive_source_span,
          helper_source_span: helper.source_span,
          algorithm_source_span: implementation.algorithm_source_span,
          scratch_declaration_source_span:
            implementation.scratch_declaration_source_span,
          source_span: callSourceSpan,
        },
        {
          op: "request_2bpp_transfer",
          source: "wDecompressScratch",
          source_wram_bank: "BANK(wDecompressScratch)",
          source_bank_argument: 1,
          source_segments: sourceSegments,
          target: destination.target,
          target_byte_offset: destination.target_byte_offset,
          target_vram_bank: vramBank,
          tile_count: helper.tile_count,
          bytes_per_tile: implementation.bytes_per_tile,
          byte_count: transferByteCount,
          request_state: {
            size: "wRequested2bppSize",
            source: "wRequested2bppSource",
            destination: "wRequested2bppDest",
            clears_size_before_copy: true,
            advances_source_and_destination: true,
          },
          chunking: {
            default_tiles_per_vblank: implementation.default_tiles_per_cycle,
            mobile_tiles_per_vblank: implementation.mobile_tiles_per_cycle,
            mobile_condition: {
              wLinkMode: "LINK_MOBILE",
              hMobile: 0,
            },
          },
          completion: {
            blocking: true,
            wait: "DelayFrame",
            until: "wRequested2bppSize == 0",
          },
          schedule: "normal_vblank_after_bg_updates_before_1bpp_and_tileset",
          saves_and_restores: [
            "hBGMapMode",
            "hROMBank",
            "hTilesPerCycle",
            "rWBK",
          ],
          destination_source_span: destination.source_span,
          destination_macro_source_span: destination.macro_source_span,
          tile_size_source_span: implementation.tile_size_source_span,
          helper_source_span: helper.source_span,
          request_source_spans: implementation.request_source_spans,
          service_source_spans: implementation.service_source_spans,
          vblank_source_span: implementation.vblank_source_span,
          request_state_source_spans: implementation.request_state_source_spans,
          source_span: callSourceSpan,
        },
      );
      resource = null;
      resourceSymbol = null;
      resourceLoadSpan = null;
      destination = null;
      index += 1;
      continue;
    }
    break;
  }
  return {
    operations,
    consumed: index - startIndex,
    vram_bank: vramBank,
  };
};

type RuntimePresentationCopyBytesImplementation = {
  source_span: RuntimePresentationSourceSpan;
  palette_size: number;
  palette_size_source_spans: RuntimePresentationSourceSpan[];
  palette_encoding_source_spans: RuntimePresentationSourceSpan[];
  destination_section_source_span: RuntimePresentationSourceSpan;
};

const certifyCopyBytesImplementation = (
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationCopyBytesImplementation => {
  const copy = loadSource("home/copy.asm", options);
  const gfxMacros = loadSource("macros/gfx.asm", options);
  const hardware = loadSource("constants/hardware.inc", options);
  const wram = loadSource("ram/wram.asm", options);
  const blocks = parseAsmBlocks([copy]);
  const spans = [
    requireExactRoutineBlock(
      blocks,
      "CopyBytes",
      ["inc b", "inc c", "jr .HandleLoop"],
      "CopyBytes exact 16-bit loop initialization",
    ),
    requireExactRoutineBlock(
      blocks,
      ".CopyByte@CopyBytes",
      ["ld a, [hli]", "ld [de], a", "inc de"],
      "CopyBytes exact ascending source/destination copy",
    ),
    requireExactRoutineBlock(
      blocks,
      ".HandleLoop@CopyBytes",
      ["dec c", "jr nz, .CopyByte", "dec b", "jr nz, .CopyByte", "ret"],
      "CopyBytes exact 16-bit byte count",
    ),
  ];
  const palettesMacroSpan = requireExactNormalizedLine(
    gfxMacros,
    'DEF palettes EQUS "* PAL_SIZE"',
    "CopyBytes palette-count expansion",
  );
  const colorSizeSpan = requireExactNormalizedLine(
    hardware,
    "def COLOR_SIZE equ 2",
    "CopyBytes RGB555 color size",
  );
  const paletteColorsSpan = requireExactNormalizedLine(
    hardware,
    "def PAL_COLORS equ 4",
    "CopyBytes palette color count",
  );
  const paletteSizeSpan = requireExactNormalizedLine(
    hardware,
    "def PAL_SIZE equ COLOR_SIZE * PAL_COLORS",
    "CopyBytes palette byte size",
  );
  const rgbMacroSpan = requireNormalizedSourceSequence(
    gfxMacros,
    [
      "MACRO? RGB",
      "rept? _NARG / 3",
      "assert_valid_rgb \\1, \\2, \\3",
      "dw palred (\\1) + palgreen (\\2) + palblue (\\3)",
      "shift 3",
      "endr",
      "ENDM",
    ],
    "CopyBytes exact RGB555 resource encoding",
  );
  const rgbChannelSpan = requireNormalizedSourceSequence(
    gfxMacros,
    [
      'DEF palred EQUS "(1 << B_COLOR_RED) *"',
      'DEF palgreen EQUS "(1 << B_COLOR_GREEN) *"',
      'DEF palblue EQUS "(1 << B_COLOR_BLUE) *"',
    ],
    "CopyBytes exact RGB555 channel packing",
  );
  const rgbBitSpan = requireNormalizedSourceSequence(
    hardware,
    [
      "def B_COLOR_RED equ COLOR_CH_WIDTH * 0",
      "def B_COLOR_GREEN equ COLOR_CH_WIDTH * 1",
      "def B_COLOR_BLUE equ COLOR_CH_WIDTH * 2",
    ],
    "CopyBytes exact RGB555 channel bit positions",
  );
  const destinationSectionSourceSpan = requireExactNormalizedLine(
    wram,
    'SECTION "GBC Video", WRAMX, ALIGN[8]',
    "CopyBytes exact banked palette destination section",
  );
  return {
    source_span: sourceSpanThrough(spans[0], spans.at(-1)!),
    palette_size: 2 * 4,
    palette_size_source_spans: [
      palettesMacroSpan,
      colorSizeSpan,
      paletteColorsSpan,
      paletteSizeSpan,
    ],
    palette_encoding_source_spans: [rgbMacroSpan, rgbChannelSpan, rgbBitSpan],
    destination_section_source_span: destinationSectionSourceSpan,
  };
};

const resolvePresentationPaletteByteCount = (
  operand: string,
  implementation: RuntimePresentationCopyBytesImplementation,
): number => {
  const match = operand.match(/^(\d+)\s+palettes$/);
  if (!match) {
    throw new Error(
      `CopyBytes count ${operand} is not a source-proven palette-count expression`,
    );
  }
  const count = Number.parseInt(match[1], 10);
  if (count <= 0 || count > 0xffff / implementation.palette_size) {
    throw new Error(`CopyBytes palette count ${count} is outside 16-bit BC`);
  }
  return count * implementation.palette_size;
};

const resolvePresentationWramDestinationSegments = (
  source: LoadedSource,
  target: string,
  byteCount: number,
  paletteSize: number,
): {
  segments: Array<{
    target: string;
    target_offset: number;
    byte_count: number;
  }>;
  declaration_source_spans: RuntimePresentationSourceSpan[];
} => {
  const declarationPattern = /^([A-Za-z_][A-Za-z0-9_]*)::\s+ds\s+(.+)$/;
  const targetIndex = source.lines.findIndex((line) => {
    const match = normalizeAsmLine(line).match(declarationPattern);
    return match?.[1] === target;
  });
  if (targetIndex < 0) {
    throw new Error(
      `CopyBytes destination ${target} has no exact WRAM allocation`,
    );
  }
  const segments: Array<{
    target: string;
    target_offset: number;
    byte_count: number;
  }> = [];
  const declarationSourceSpans: RuntimePresentationSourceSpan[] = [];
  let remaining = byteCount;
  let lineIndex = targetIndex;
  while (remaining > 0) {
    while (
      lineIndex < source.lines.length &&
      normalizeAsmLine(source.lines[lineIndex]).length === 0
    ) {
      lineIndex += 1;
    }
    const normalized = normalizeAsmLine(source.lines[lineIndex] ?? "");
    const declaration = normalized.match(declarationPattern);
    if (!declaration) {
      throw new Error(
        `CopyBytes destination ${target} crosses an unproved WRAM boundary after ${byteCount - remaining} bytes at ${source.file}:${lineIndex + 1}`,
      );
    }
    const paletteCount = declaration[2].match(/^(\d+)\s+palettes$/);
    const directCount = declaration[2].match(/^\$([0-9a-f]+)$/i);
    const capacity = paletteCount
      ? Number.parseInt(paletteCount[1], 10) * paletteSize
      : directCount
        ? Number.parseInt(directCount[1], 16)
        : null;
    if (!capacity || capacity <= 0) {
      throw new Error(
        `CopyBytes destination ${declaration[1]} has unsupported allocation ${declaration[2]}`,
      );
    }
    const count = Math.min(remaining, capacity);
    segments.push({
      target: declaration[1],
      target_offset: 0,
      byte_count: count,
    });
    declarationSourceSpans.push({
      file: source.file,
      start_line: lineIndex + 1,
      end_line: lineIndex + 1,
    });
    remaining -= count;
    lineIndex += 1;
  }
  return { segments, declaration_source_spans: declarationSourceSpans };
};

const compileIntroSceneCopyBytesPrefix = (
  firstScene: readonly RuntimePresentationAsmInstruction[],
  startIndex: number,
  intro: LoadedSource,
  destinationBank: string,
  options: BuildRuntimeTitlePresentationProgramOptions,
): { operations: RuntimePresentationOperation[]; consumed: number } => {
  const implementation = certifyCopyBytesImplementation(options);
  const blocks = parseAsmBlocks([intro]);
  const constants = parseAsmConstants(
    CONSTANT_SOURCE_FILES.map((file) => loadSource(file, options)),
  );
  const wram = loadSource("ram/wram.asm", options);
  const operations: RuntimePresentationOperation[] = [];
  let index = startIndex;
  while (index + 3 < firstScene.length) {
    const sourceLoad = firstScene[index];
    const destinationLoad = firstScene[index + 1];
    const lengthLoad = firstScene[index + 2];
    const call = firstScene[index + 3];
    if (
      sourceLoad.opcode !== "ld" ||
      sourceLoad.args[0] !== "hl" ||
      !sourceLoad.args[1] ||
      destinationLoad.opcode !== "ld" ||
      destinationLoad.args[0] !== "de" ||
      !destinationLoad.args[1] ||
      lengthLoad.opcode !== "ld" ||
      lengthLoad.args[0] !== "bc" ||
      !lengthLoad.args[1] ||
      instructionSignature(call) !== "call CopyBytes"
    ) {
      break;
    }
    const resource = parseRgb555IncludedResource(
      sourceLoad.args[1],
      blocks,
      constants,
      (file) => loadSource(file, options),
    );
    const byteCount = resolvePresentationPaletteByteCount(
      lengthLoad.args[1],
      implementation,
    );
    if (resource.bytes.length < byteCount) {
      throw new Error(
        `CopyBytes source ${sourceLoad.args[1]} resource ${resource.path} has ${resource.bytes.length} bytes but the source requests ${byteCount}`,
      );
    }
    const destination = resolvePresentationWramDestinationSegments(
      wram,
      destinationLoad.args[1],
      byteCount,
      implementation.palette_size,
    );
    operations.push({
      op: "copy_memory",
      source: sourceLoad.args[1],
      source_address_space: "current_rom_bank",
      resource: resource.path,
      resource_kind: "palette",
      resource_byte_count: resource.bytes.length,
      source_offset: 0,
      values: resource.bytes.slice(0, byteCount),
      target: destinationLoad.args[1],
      target_address_space: "wramx",
      destination_bank: destinationBank,
      destination_segments: destination.segments,
      byte_count: byteCount,
      direction: "ascending",
      overlap: "disjoint_rom_and_wram_address_spaces",
      resource_source_span: resource.include_source_span,
      resource_data_source_span: resource.data_source_span,
      palette_encoding_source_spans:
        implementation.palette_encoding_source_spans,
      palette_size_source_spans: implementation.palette_size_source_spans,
      destination_declaration_source_spans:
        destination.declaration_source_spans,
      destination_section_source_span:
        implementation.destination_section_source_span,
      implementation_source_span: implementation.source_span,
      source_span: sourceSpanThrough(sourceLoad.source_span, call.source_span),
    });
    index += 4;
  }
  return { operations, consumed: index - startIndex };
};

function certifyCrystalIntroSubprogramFrontier(
  options: BuildRuntimeTitlePresentationProgramOptions,
  controlFlow: RuntimePresentationControlFlow,
): RuntimePresentationEmissionFrontier {
  const intro = loadSource("engine/movie/intro.asm", options);
  const joypad = loadSource("home/joypad.asm", options);
  const clearSprites = loadSource("home/clear_sprites.asm", options);
  const delay = loadSource("home/delay.asm", options);
  const blocks = parseAsmBlocks([intro, joypad, clearSprites, delay]);

  requireExactRoutineBlock(
    blocks,
    "CrystalIntro",
    [
      "ldh a, [rWBK]",
      "push af",
      "ld a, BANK(wGBCPalettes)",
      "ldh [rWBK], a",
      "ldh a, [hInMenu]",
      "push af",
      "ldh a, [hVBlank]",
      "push af",
      "call .InitRAMAddrs",
    ],
    "CrystalIntro exact saved-state entry",
  );
  requireExactRoutineBlock(
    blocks,
    ".loop@CrystalIntro",
    [
      "call JoyTextDelay",
      "ldh a, [hJoyLast]",
      "and PAD_BUTTONS",
      "jr nz, .ShutOffMusic",
      "ld a, [wJumptableIndex]",
      "bit JUMPTABLE_EXIT_F, a",
      "jr nz, .done",
      "call IntroSceneJumper",
      "farcall PlaySpriteAnimations",
      "call DelayFrame",
      "jp .loop",
    ],
    "CrystalIntro input, scene, sprite, VBlank, and repeat order",
  );
  requireExactRoutineBlock(
    blocks,
    ".ShutOffMusic@CrystalIntro",
    ["ld de, MUSIC_NONE", "call PlayMusic"],
    "CrystalIntro button-cancel music stop",
  );
  requireExactRoutineBlock(
    blocks,
    ".done@CrystalIntro",
    [
      "call ClearBGPalettes",
      "call ClearSprites",
      "call ClearTilemap",
      "xor a",
      "ldh [hSCX], a",
      "ldh [hSCY], a",
      "ld a, 7",
      "ldh [hWX], a",
      "ld a, SCREEN_HEIGHT_PX",
      "ldh [hWY], a",
      "pop af",
      "ldh [hVBlank], a",
      "pop af",
      "ldh [hInMenu], a",
      "pop af",
      "ldh [rWBK], a",
      "ret",
    ],
    "CrystalIntro exact display reset and saved-state restoration",
  );
  requireExactRoutineBlock(
    blocks,
    ".InitRAMAddrs@CrystalIntro",
    [
      "assert VBLANK_NORMAL == 0",
      "xor a",
      "ldh [hVBlank], a",
      "ld a, TRUE",
      "ldh [hInMenu], a",
      "xor a",
      "ldh [hMapAnims], a",
      "ld [wJumptableIndex], a",
      "ret",
    ],
    "CrystalIntro exact loop-state initialization",
  );
  requireExactRoutineBlock(
    blocks,
    "JoyTextDelay",
    [
      "call GetJoypad",
      "ldh a, [hInMenu]",
      "and a",
      "ldh a, [hJoyPressed]",
      "jr z, .ok",
      "ldh a, [hJoyDown]",
    ],
    "CrystalIntro JoyTextDelay pressed-versus-held selection",
  );
  requireExactRoutineBlock(
    blocks,
    ".ok@JoyTextDelay",
    [
      "ldh [hJoyLast], a",
      "ldh a, [hJoyPressed]",
      "and a",
      "jr z, .checkframedelay",
      "ld a, 15",
      "ld [wTextDelayFrames], a",
      "ret",
    ],
    "CrystalIntro JoyTextDelay pressed-input repeat reset",
  );
  requireExactRoutineBlock(
    blocks,
    ".checkframedelay@JoyTextDelay",
    [
      "ld a, [wTextDelayFrames]",
      "and a",
      "jr z, .restartframedelay",
      "xor a",
      "ldh [hJoyLast], a",
      "ret",
    ],
    "CrystalIntro JoyTextDelay repeat suppression",
  );
  requireExactRoutineBlock(
    blocks,
    ".restartframedelay@JoyTextDelay",
    ["ld a, 5", "ld [wTextDelayFrames], a", "ret"],
    "CrystalIntro JoyTextDelay repeat restart",
  );
  const clearOamSpan = requireExactRoutineBlock(
    blocks,
    "ClearSprites",
    ["ld hl, wShadowOAM", "ld b, wShadowOAMEnd - wShadowOAM", "xor a"],
    "CrystalIntro exact OAM-clear setup",
  );
  const clearOamLoopSpan = requireExactRoutineBlock(
    blocks,
    ".loop@ClearSprites",
    ["ld [hli], a", "dec b", "jr nz, .loop", "ret"],
    "CrystalIntro exact OAM-clear loop",
  );
  certifyDelayFrames(blocks);
  certifyClearBgPalettesOperations(options);
  const clearTileOperations = certifyClearTilemapOperations(options);

  requireExactRoutineBlock(
    blocks,
    "IntroSceneJumper",
    ["jumptable IntroScenes, wJumptableIndex"],
    "CrystalIntro source scene dispatcher",
  );
  requireExactRoutineBlock(
    blocks,
    "NextIntroScene",
    ["ld hl, wJumptableIndex", "inc [hl]", "ret"],
    "CrystalIntro exact scene-index increment",
  );
  requireExactRoutineBlock(
    blocks,
    ".done@IntroScene28",
    ["ld hl, wJumptableIndex", "set JUMPTABLE_EXIT_F, [hl]", "ret"],
    "CrystalIntro terminal scene exit-bit write",
  );
  const sceneTable = parseWordTable(
    intro,
    "IntroScenes:",
    "NextIntroScene:",
    false,
  );
  const exactSceneEntries = Array.from(
    { length: 28 },
    (_, index) => `IntroScene${index + 1}`,
  );
  const sceneDomain = controlFlow.indirect_tables.find(
    (candidate) => candidate.table === "IntroScenes",
  )?.index_domain;
  if (
    sceneTable.entries.join(",") !== exactSceneEntries.join(",") ||
    sceneDomain?.values.join(",") !==
      Array.from({ length: 28 }, (_, index) => index).join(",")
  ) {
    throw new Error(
      "CrystalIntro scene dispatcher has no exact source-derived 0..27 domain",
    );
  }
  if (
    controlFlow.sprite_diagnostics.length !== 0 ||
    !controlFlow.sprite_programs.some(
      (program) =>
        program.allocation_source_span.file === "engine/movie/intro.asm",
    )
  ) {
    throw new Error(
      "CrystalIntro scene dispatcher has no closed source-derived sprite program set",
    );
  }

  const firstScene = blocks.get("IntroScene1")?.instructions ?? [];
  const expectedPrefix = [
    "call Intro_ClearBGPals",
    "call ClearSprites",
    "call ClearTilemap",
    "xor a",
    "ldh [hBGMapMode], a",
    "ld a, $1",
    "ldh [rVBK], a",
  ];
  const reachedPrefix = firstScene
    .slice(0, expectedPrefix.length)
    .map(instructionSignature);
  if (reachedPrefix.join("\0") !== expectedPrefix.join("\0")) {
    throw new Error(
      `CrystalIntro first scene prefix is not exact; reached ${reachedPrefix.join(" -> ")}`,
    );
  }
  const bgModeRun = compileAccumulatorHighMemoryWriteRun(
    firstScene,
    3,
    options,
  );
  const vramBankRun = compileAccumulatorHighMemoryWriteRun(
    firstScene,
    5,
    options,
  );
  if (
    !bgModeRun ||
    bgModeRun.consumed !== 2 ||
    !vramBankRun ||
    vramBankRun.consumed !== 2
  ) {
    throw new Error(
      "CrystalIntro first scene register writes have no exact accumulator data flow",
    );
  }
  const compiledPrefix: RuntimePresentationOperation[] = [
    ...certifyIntroClearBgPalettesOperations(options),
    {
      op: "fill_memory",
      target: "wShadowOAM",
      byte_count: 160,
      value: 0,
      direction: "ascending",
      bank: { select: "current", restore: false },
      condition: { source: null, predicate: "always", source_span: null },
      source_span: sourceSpanThrough(clearOamSpan, clearOamLoopSpan),
    },
    ...clearTileOperations,
    ...bgModeRun.operations,
    ...vramBankRun.operations,
  ];
  const decompressionPrefix = compileIntroSceneDecompressionPrefix(
    firstScene,
    expectedPrefix.length,
    1,
    intro,
    options,
  );
  if (decompressionPrefix.operations.length === 0) {
    const instruction = firstScene[expectedPrefix.length];
    return {
      reason: "missing_runtime_operation",
      block: "IntroScene1",
      target: runtimePresentationInstructionTarget(instruction),
      opcode: instruction.opcode,
      args: instruction.args,
      source_span: instruction.source_span,
      compiled_prefix: {
        source_entry: "CrystalIntro",
        block: "IntroScene1",
        operations: compiledPrefix,
      },
    };
  }
  compiledPrefix.push(...decompressionPrefix.operations);

  let cursor = expectedPrefix.length + decompressionPrefix.consumed;
  const bankRead = firstScene[cursor];
  const bankPush = firstScene[cursor + 1];
  const bankValue = firstScene[cursor + 2];
  const bankWrite = firstScene[cursor + 3];
  const bankMatch = bankValue?.args[1]?.match(
    /^BANK\(([A-Za-z_.][A-Za-z0-9_.@]*)\)$/,
  );
  if (
    instructionSignature(bankRead) !== "ldh a, [rWBK]" ||
    instructionSignature(bankPush) !== "push af" ||
    bankValue?.opcode !== "ld" ||
    bankValue.args[0] !== "a" ||
    !bankMatch ||
    instructionSignature(bankWrite) !== "ldh [rWBK], a"
  ) {
    throw new Error(
      `CrystalIntro first scene has no exact source-derived WRAM bank save/select scope before its next transfer`,
    );
  }
  const wram = loadSource("ram/wram.asm", options);
  const bankSymbolSpan = findAsmSymbolDeclarationSpan(bankMatch[1], [wram]);
  if (!bankSymbolSpan) {
    throw new Error(
      `CrystalIntro first scene WRAM bank operand ${bankValue.args[1]} has no exact source declaration`,
    );
  }
  compiledPrefix.push(
    {
      op: "save_memory_byte",
      source: "rWBK",
      storage: { kind: "cpu_stack", register_pair: "af" },
      restore_required: true,
      source_span: sourceSpanThrough(
        bankRead.source_span,
        bankPush.source_span,
      ),
    },
    {
      op: "write_memory_byte",
      target: "rWBK",
      address_space: "hardware_register",
      value: bankValue.args[1],
      condition: { source: null, predicate: "always", source_span: null },
      value_source_span: bankSymbolSpan,
      source_span: sourceSpanThrough(
        bankValue.source_span,
        bankWrite.source_span,
      ),
    },
  );
  cursor += 4;

  const copyPrefix = compileIntroSceneCopyBytesPrefix(
    firstScene,
    cursor,
    intro,
    bankValue.args[1],
    options,
  );
  if (copyPrefix.operations.length === 0) {
    const instruction = firstScene[cursor];
    return {
      reason: "missing_runtime_operation",
      block: "IntroScene1",
      target: runtimePresentationInstructionTarget(instruction),
      opcode: instruction.opcode,
      args: instruction.args,
      source_span: instruction.source_span,
      compiled_prefix: {
        source_entry: "CrystalIntro",
        block: "IntroScene1",
        operations: compiledPrefix,
      },
    };
  }
  compiledPrefix.push(...copyPrefix.operations);
  cursor += copyPrefix.consumed;

  const bankPop = firstScene[cursor];
  const bankRestore = firstScene[cursor + 1];
  if (
    instructionSignature(bankPop) !== "pop af" ||
    instructionSignature(bankRestore) !== "ldh [rWBK], a"
  ) {
    throw new Error(
      "CrystalIntro first scene does not exactly restore its saved WRAM bank after CopyBytes",
    );
  }
  compiledPrefix.push({
    op: "restore_memory_byte",
    target: "rWBK",
    storage: { kind: "cpu_stack", register_pair: "af" },
    matches_save_source_span: sourceSpanThrough(
      bankRead.source_span,
      bankPush.source_span,
    ),
    source_span: sourceSpanThrough(
      bankPop.source_span,
      bankRestore.source_span,
    ),
  });
  cursor += 2;

  while (cursor < firstScene.length) {
    const writeRun = compileAccumulatorHighMemoryWriteRun(
      firstScene,
      cursor,
      options,
    );
    if (!writeRun) break;
    compiledPrefix.push(...writeRun.operations);
    cursor += writeRun.consumed;
  }
  if (
    runtimePresentationInstructionTarget(firstScene[cursor]) ===
    "ClearSpriteAnims"
  ) {
    compiledPrefix.push(
      compileClearSpriteAnimsCall(firstScene[cursor], options),
    );
    cursor += 1;
  }
  if (
    runtimePresentationInstructionTarget(firstScene[cursor]) ===
    "Intro_SetCGBPalUpdate"
  ) {
    compiledPrefix.push(
      compileAccumulatorWriteSubprogramCall(
        firstScene[cursor],
        "Intro_SetCGBPalUpdate",
        blocks,
        options,
      ),
    );
    cursor += 1;
  }
  const sceneStateWrites = compileAccumulatorWramWriteRun(
    firstScene,
    cursor,
    options,
  );
  if (sceneStateWrites) {
    compiledPrefix.push(...sceneStateWrites.operations);
    cursor += sceneStateWrites.consumed;
  }
  if (
    runtimePresentationInstructionTarget(firstScene[cursor]) ===
    "NextIntroScene"
  ) {
    compiledPrefix.push(
      compileIncrementMemoryByteSubprogramCall(
        firstScene[cursor],
        "NextIntroScene",
        blocks,
        options,
      ),
    );
    cursor += 1;
  }
  if (instructionSignature(firstScene[cursor]) === "ret") {
    compiledPrefix.push({
      op: "return",
      source_span: firstScene[cursor].source_span,
    });
    cursor += 1;
  }
  let frontierBlock = "IntroScene1";
  let nextCall = firstScene[cursor];
  if (!nextCall) {
    frontierBlock = ".loop@CrystalIntro";
    const loopInstructions = blocks.get(frontierBlock)?.instructions ?? [];
    const schedulerIndex = loopInstructions.findIndex(
      (instruction) =>
        runtimePresentationInstructionTarget(instruction) ===
        "PlaySpriteAnimations",
    );
    const schedulerCall = loopInstructions[schedulerIndex];
    const delayCall = loopInstructions[schedulerIndex + 1];
    const repeatJump = loopInstructions[schedulerIndex + 2];
    const resetOperation = [...compiledPrefix]
      .reverse()
      .find(
        (operation) =>
          operation.op === "fill_memory" &&
          operation.target === "wSpriteAnimData",
      );
    if (
      schedulerIndex < 0 ||
      loopInstructions.filter(
        (instruction) =>
          runtimePresentationInstructionTarget(instruction) ===
          "PlaySpriteAnimations",
      ).length !== 1 ||
      !resetOperation
    ) {
      throw new Error(
        "CrystalIntro first scene return has no exact central PlaySpriteAnimations continuation",
      );
    }
    compiledPrefix.push(
      compileEmptySpriteSchedulerCall(
        schedulerCall,
        resetOperation,
        options,
      ),
    );
    if (
      instructionSignature(delayCall) !== "call DelayFrame" ||
      instructionSignature(repeatJump) !== "jp .loop" ||
      instructionTarget(blocks.get(frontierBlock)!, repeatJump, blocks) !==
        frontierBlock
    ) {
      throw new Error(
        "CrystalIntro central sprite scheduler is not followed by one DelayFrame and its exact loop jump",
      );
    }
    compiledPrefix.push(
      {
        op: "wait_frames",
        frames: 1,
        condition: {
          source: null,
          predicate: "always",
          source_span: null,
        },
        implementation_source_span: delayFramesImplementationSpan,
        invocation: {
          call_form: "call",
          target: "DelayFrame",
          stack_effect: "push_return_address_then_ret",
          register_result: {
            a: 0,
            bc: "unchanged",
            de: "unchanged",
            hl: "unchanged",
            flags: {
              zero: true,
              subtract: false,
              half_carry: false,
              carry: false,
            },
          },
          source_span: delayCall.source_span,
        },
        source_span: delayCall.source_span,
      },
      {
        op: "jump",
        target: frontierBlock,
        source_span: repeatJump.source_span,
      },
    );
    nextCall = loopInstructions[0];
  }
  const nextTarget = runtimePresentationInstructionTarget(nextCall);
  const reason = ["call", "callfar", "farcall"].includes(nextCall.opcode)
    ? "missing_subprogram_contract"
    : "missing_runtime_operation";
  return {
    reason,
    block: frontierBlock,
    target: nextTarget,
    opcode: nextCall.opcode,
    args: nextCall.args,
    source_span: nextCall.source_span,
    compiled_prefix: {
      source_entry: "CrystalIntro",
      block: "IntroScene1",
      operations: compiledPrefix,
    },
  };
}

const RUNTIME_PRESENTATION_SOURCE_SUBPROGRAM_BOUNDARIES: Record<
  string,
  RuntimePresentationSourceSubprogramBoundary
> = {
  SplashScreen: {
    accepted_call_forms: ["callfar"],
    certify: certifySplashScreenSubprogram,
  },
  CrystalIntro: {
    accepted_call_forms: ["farcall"],
    certify_frontier: certifyCrystalIntroSubprogramFrontier,
  },
};

export type RuntimePresentationEmissionFrontier = {
  reason:
    | "missing_host_effect_contract"
    | "missing_subprogram_contract"
    | "missing_runtime_operation";
  block: string;
  target: string | null;
  opcode: string;
  args: string[];
  source_span: RuntimePresentationSourceSpan;
  compiled_prefix?: {
    source_entry: string;
    block: string;
    operations: RuntimePresentationOperation[];
  };
};

export type RuntimePresentationEmissionCheckpoint = {
  entrypoints: RuntimePresentationProgram["entrypoints"];
  blocks: RuntimePresentationProgram["blocks"];
  host_effects: RuntimePresentationProgram["host_effects"];
  subprograms: RuntimePresentationProgram["subprograms"];
  frontier: RuntimePresentationEmissionFrontier | null;
};

export function analyzeRuntimeTitlePresentationEmission(
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationEmissionCheckpoint {
  const controlFlow = requireClosedRuntimePresentationControlFlow(options);
  const blocks: RuntimePresentationProgram["blocks"] = {};
  const hostEffects: RuntimePresentationProgram["host_effects"] = [];
  const hostEffectIds = new Set<string>();
  const subprograms: RuntimePresentationProgram["subprograms"] = [];
  const subprogramIds = new Set<string>();
  const pending = RUNTIME_PRESENTATION_ENTRYPOINTS.map(
    (entrypoint) => controlFlow.entrypoints[entrypoint],
  );
  const visited = new Set<string>();

  while (pending.length > 0) {
    const blockId = pending.shift()!;
    if (visited.has(blockId)) continue;
    visited.add(blockId);
    const sourceBlock = controlFlow.blocks[blockId];
    if (!sourceBlock) {
      throw new Error(
        `Runtime presentation emission reaches missing block ${blockId}`,
      );
    }
    const operations: RuntimePresentationOperation[] = [];
    let activeCarryResult:
      RuntimePresentationCallableSubprogram["result"] | null = null;
    blocks[blockId] = {
      source_span: sourceBlock.source_span,
      operations,
    };
    for (
      let instructionIndex = 0;
      instructionIndex < sourceBlock.instructions.length;
      instructionIndex += 1
    ) {
      const instruction = sourceBlock.instructions[instructionIndex];
      const highMemoryWrites = compileAccumulatorHighMemoryWriteRun(
        sourceBlock.instructions,
        instructionIndex,
        options,
      );
      if (highMemoryWrites) {
        operations.push(...highMemoryWrites.operations);
        if (instruction.opcode === "xor") activeCarryResult = null;
        instructionIndex += highMemoryWrites.consumed - 1;
        continue;
      }
      const target = runtimePresentationInstructionTarget(instruction);
      const callForm = ["call", "callfar", "farcall"].includes(
        instruction.opcode,
      )
        ? (instruction.opcode as RuntimePresentationHostEffectCallForm)
        : null;
      const subprogramBoundary = target
        ? RUNTIME_PRESENTATION_SOURCE_SUBPROGRAM_BOUNDARIES[target]
        : undefined;
      if (
        callForm &&
        subprogramBoundary?.accepted_call_forms.includes(callForm)
      ) {
        if (subprogramBoundary.certify_frontier) {
          return {
            entrypoints: controlFlow.entrypoints,
            blocks,
            host_effects: hostEffects,
            subprograms,
            frontier: subprogramBoundary.certify_frontier(options, controlFlow),
          };
        }
        if (!subprogramBoundary.certify) {
          throw new Error(
            `Runtime presentation subprogram ${String(target)} has no source certification implementation`,
          );
        }
        const contract = subprogramBoundary.certify(options, controlFlow);
        if (
          contract.source_entry !== target ||
          !contract.accepted_call_forms.includes(callForm)
        ) {
          throw new Error(
            `Runtime presentation subprogram contract ${contract.id} does not accept ${callForm} ${String(target)}`,
          );
        }
        if (!subprogramIds.has(contract.id)) {
          subprogramIds.add(contract.id);
          subprograms.push(contract);
        }
        operations.push({
          op: "call_subprogram",
          program: contract.id,
          result: contract.result.name,
          source_span: instruction.source_span,
        });
        activeCarryResult = contract.result;
        continue;
      }
      if (
        (instruction.opcode === "jr" || instruction.opcode === "jp") &&
        instruction.args.length === 2 &&
        ["c", "nc"].includes(instruction.args[0]) &&
        activeCarryResult &&
        target &&
        Object.hasOwn(controlFlow.blocks, target)
      ) {
        const expectedCarry = instruction.args[0] === "c" ? 1 : 0;
        const matchingCases = activeCarryResult.domain.filter(
          (candidate) => candidate.value === expectedCarry,
        );
        if (matchingCases.length !== 1) {
          throw new Error(
            `Runtime presentation ${instruction.args[0]} branch at ${instruction.source_span.file}:` +
              `${instruction.source_span.start_line} cannot resolve one exact case from ${activeCarryResult.name}`,
          );
        }
        operations.push({
          op: "branch_result",
          result: activeCarryResult.name,
          equals: matchingCases[0].id,
          target,
          source_span: instruction.source_span,
        });
        pending.push(target);
        activeCarryResult = null;
        continue;
      }
      const external = controlFlow.external_calls.find(
        (candidate) =>
          candidate.target === target &&
          candidate.source_span.file === instruction.source_span.file &&
          candidate.source_span.start_line ===
            instruction.source_span.start_line,
      );
      if (external) {
        const sourceBoundary =
          RUNTIME_PRESENTATION_SOURCE_OPERATION_BOUNDARIES[target!];
        if (
          sourceBoundary &&
          sourceBoundary.accepted_call_forms.includes(external.call_form)
        ) {
          operations.push(...sourceBoundary.certify(options));
          continue;
        }
        const boundary = RUNTIME_PRESENTATION_HOST_EFFECT_BOUNDARIES[target!];
        if (
          boundary &&
          boundary.accepted_call_forms.includes(external.call_form)
        ) {
          const contract = boundary.certify(options);
          if (
            contract.call_target !== external.target ||
            !contract.accepted_call_forms.includes(external.call_form)
          ) {
            throw new Error(
              `Runtime presentation host-effect contract ${contract.id} does not accept ` +
                `${external.call_form} ${external.target}`,
            );
          }
          if (!hostEffectIds.has(contract.id)) {
            hostEffectIds.add(contract.id);
            hostEffects.push(contract);
          }
          operations.push({
            op: "host_effect",
            effect: contract.id,
            result: contract.result.name,
            args: [],
            source_span: instruction.source_span,
          });
          continue;
        }
        return {
          entrypoints: controlFlow.entrypoints,
          blocks,
          host_effects: hostEffects,
          subprograms,
          frontier: {
            reason: "missing_host_effect_contract",
            block: blockId,
            target: external.target,
            opcode: instruction.opcode,
            args: instruction.args,
            source_span: instruction.source_span,
          },
        };
      }
      if (
        (instruction.opcode === "jp" || instruction.opcode === "jr") &&
        instruction.args.length === 1 &&
        target &&
        Object.hasOwn(controlFlow.blocks, target)
      ) {
        operations.push({
          op: "jump",
          target,
          source_span: instruction.source_span,
        });
        pending.push(target);
        continue;
      }
      if (instruction.opcode === "ret" && instruction.args.length === 0) {
        operations.push({
          op: "return",
          source_span: instruction.source_span,
        });
        continue;
      }
      if (
        target &&
        ["call", "callfar", "farcall"].includes(instruction.opcode) &&
        Object.hasOwn(controlFlow.blocks, target)
      ) {
        return {
          entrypoints: controlFlow.entrypoints,
          blocks,
          host_effects: hostEffects,
          subprograms,
          frontier: {
            reason: "missing_subprogram_contract",
            block: blockId,
            target,
            opcode: instruction.opcode,
            args: instruction.args,
            source_span: instruction.source_span,
          },
        };
      }
      return {
        entrypoints: controlFlow.entrypoints,
        blocks,
        host_effects: hostEffects,
        subprograms,
        frontier: {
          reason: "missing_runtime_operation",
          block: blockId,
          target,
          opcode: instruction.opcode,
          args: instruction.args,
          source_span: instruction.source_span,
        },
      };
    }
    if (sourceBlock.fallthrough) pending.push(sourceBlock.fallthrough);
    pending.push(...sourceBlock.direct_targets);
  }

  return {
    entrypoints: controlFlow.entrypoints,
    blocks,
    host_effects: hostEffects,
    subprograms,
    frontier: null,
  };
}

export function buildRuntimeTitlePresentationProgram(
  options: BuildRuntimeTitlePresentationProgramOptions,
): RuntimePresentationProgram {
  const checkpoint = analyzeRuntimeTitlePresentationEmission(options);
  const frontier = checkpoint.frontier;
  if (frontier) {
    const location =
      `${frontier.source_span.file}:` + `${frontier.source_span.start_line}`;
    if (frontier.reason === "missing_host_effect_contract") {
      throw new Error(
        `Reachable presentation host effect ${String(frontier.target)} at ${location} ` +
          "has no exact typed host-effect contract",
      );
    }
    if (frontier.reason === "missing_subprogram_contract") {
      throw new Error(
        `Reachable presentation source call ${String(frontier.target)} at ${location} ` +
          "has no exact typed subprogram contract",
      );
    }
    throw new Error(
      `Reachable presentation instruction ${frontier.opcode} ` +
        `${frontier.args.join(", ")} at ${location} ` +
        "has no exact typed runtime operation",
    );
  }

  throw new Error(
    "Runtime presentation program reached the resource/audio/text/host-effect catalog boundary without an exact source-derived catalog",
  );
}

const RESOURCE_SOURCE_FILES = [
  "engine/movie/title.asm",
  "engine/movie/splash.asm",
  "engine/movie/intro.asm",
  "engine/menus/init_gender.asm",
  "engine/rtc/timeset.asm",
  "engine/gfx/player_gfx.asm",
  "gfx/font.asm",
  "gfx/misc.asm",
  "gfx/pics.asm",
  "engine/gfx/cgb_layouts.asm",
  "data/trainers/palettes.asm",
] as const;

const DIRECT_RESOURCE_PATHS = [
  "gfx/title/suicune.2bpp.lz",
  "gfx/title/logo.2bpp.lz",
  "gfx/title/crystal.2bpp.lz",
  "gfx/title/title.pal",
  "gfx/splash/copyright.2bpp",
  "gfx/splash/ditto.2bpp.lz",
  "gfx/splash/ditto.pal",
  "gfx/splash/ditto_fade.pal",
  "gfx/splash/gamefreak_presents.1bpp",
  "gfx/splash/gamefreak_logo.1bpp",
  "gfx/new_game/gender_screen.pal",
  "gfx/new_game/gender_screen.2bpp",
  "gfx/new_game/timeset_bg.1bpp",
  "gfx/new_game/up_arrow.1bpp",
  "gfx/new_game/down_arrow.1bpp",
  "gfx/new_game/shrink1.2bpp.lz",
  "gfx/new_game/shrink2.2bpp.lz",
  "gfx/player/chris.2bpp",
  "gfx/player/kris.2bpp",
  "gfx/trainers/oak.2bpp.lz",
  "gfx/trainers/oak.gbcpal",
  "gfx/pokemon/wooper/front.animated.2bpp.lz",
  "gfx/font/font_extra.2bpp",
  "gfx/font/font.1bpp",
  "gfx/frames/1.1bpp",
  "gfx/frames/2.1bpp",
  "gfx/frames/3.1bpp",
  "gfx/frames/4.1bpp",
  "gfx/frames/5.1bpp",
  "gfx/frames/6.1bpp",
  "gfx/frames/7.1bpp",
  "gfx/frames/8.1bpp",
] as const;

const INTRO_RESOURCE_PATHS = [
  "gfx/intro/fade.pal",
  "gfx/intro/unown_1.pal",
  "gfx/intro/unown_2.pal",
  "gfx/intro/suicune_run.2bpp.lz",
  "gfx/intro/pichu_wooper.2bpp.lz",
  "gfx/intro/background.2bpp.lz",
  "gfx/intro/background.tilemap.lz",
  "gfx/intro/background.attrmap.lz",
  "gfx/intro/background.pal",
  "gfx/intro/unowns.2bpp.lz",
  "gfx/intro/pulse.2bpp.lz",
  "gfx/intro/unown_a.tilemap.lz",
  "gfx/intro/unown_a.attrmap.lz",
  "gfx/intro/unown_hi.tilemap.lz",
  "gfx/intro/unown_hi.attrmap.lz",
  "gfx/intro/unowns.tilemap.lz",
  "gfx/intro/unowns.attrmap.lz",
  "gfx/intro/unowns.pal",
  "gfx/intro/crystal_unowns.2bpp.lz",
  "gfx/intro/crystal_unowns.tilemap.lz",
  "gfx/intro/crystal_unowns.attrmap.lz",
  "gfx/intro/crystal_unowns.pal",
  "gfx/intro/suicune_close.2bpp.lz",
  "gfx/intro/suicune_close.tilemap.lz",
  "gfx/intro/suicune_close.attrmap.lz",
  "gfx/intro/suicune_close.pal",
  "gfx/intro/suicune_jump.2bpp.lz",
  "gfx/intro/suicune_back.2bpp.lz",
  "gfx/intro/suicune_jump.tilemap.lz",
  "gfx/intro/suicune_jump.attrmap.lz",
  "gfx/intro/suicune_back.tilemap.lz",
  "gfx/intro/suicune_back.attrmap.lz",
  "gfx/intro/suicune.pal",
  "gfx/intro/unown_back.2bpp.lz",
  "gfx/intro/grass1.2bpp",
  "gfx/intro/grass2.2bpp",
  "gfx/intro/grass3.2bpp",
  "gfx/intro/grass4.2bpp",
] as const;

const REQUIRED_AUDIO: ReadonlyArray<{
  id: string;
  kind: "music" | "sound_effect" | "cry" | "silence";
}> = [
  { id: "MUSIC_NONE", kind: "silence" },
  { id: "MUSIC_TITLE", kind: "music" },
  { id: "MUSIC_MAIN_MENU", kind: "music" },
  { id: "MUSIC_CRYSTAL_OPENING", kind: "music" },
  { id: "MUSIC_ROUTE_30", kind: "music" },
  { id: "MUSIC_MOBILE_ADAPTER_MENU", kind: "music" },
  { id: "SFX_TITLE_SCREEN_ENTRANCE", kind: "sound_effect" },
  { id: "SFX_GAME_FREAK_PRESENTS", kind: "sound_effect" },
  { id: "SFX_DITTO_BOUNCE", kind: "sound_effect" },
  { id: "SFX_DITTO_POP_UP", kind: "sound_effect" },
  { id: "SFX_DITTO_TRANSFORM", kind: "sound_effect" },
  { id: "SFX_INTRO_UNOWN_1", kind: "sound_effect" },
  { id: "SFX_INTRO_UNOWN_2", kind: "sound_effect" },
  { id: "SFX_INTRO_UNOWN_3", kind: "sound_effect" },
  { id: "SFX_INTRO_SUICUNE_2", kind: "sound_effect" },
  { id: "SFX_INTRO_SUICUNE_3", kind: "sound_effect" },
  { id: "SFX_INTRO_SUICUNE_4", kind: "sound_effect" },
  { id: "SFX_INTRO_PICHU", kind: "sound_effect" },
  { id: "SFX_INTRO_WHOOSH", kind: "sound_effect" },
  { id: "SFX_ESCAPE_ROPE", kind: "sound_effect" },
  { id: "CRY_WOOPER", kind: "cry" },
] as const;

const TEXT_SOURCE_FILES = [
  "engine/menus/intro_menu.asm",
  "engine/menus/main_menu.asm",
  "engine/menus/delete_save.asm",
  "engine/rtc/reset_password.asm",
  "engine/rtc/restart_clock.asm",
  "engine/rtc/timeset.asm",
  "engine/menus/init_gender.asm",
] as const;

const normalizeAsmLine = (line: string): string => {
  let inQuotes = false;
  let value = line;
  for (let index = 0; index < line.length; index += 1) {
    if (line[index] === '"') {
      inQuotes = !inQuotes;
    } else if (line[index] === ";" && !inQuotes) {
      value = line.slice(0, index);
      break;
    }
  }
  return value.trim().replace(/\s+/g, " ");
};

const labelPattern = (label: string): RegExp =>
  new RegExp(`^${label.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}::?\\s*$`);

const loadSource = (
  file: string,
  options: BuildRuntimeTitlePresentationProgramOptions,
): LoadedSource => ({
  file,
  lines: (options.readSource
    ? options.readSource(file)
    : fs.readFileSync(path.join(options.disassemblyRoot, file), "utf8")
  ).split(/\r?\n/),
});

const findLabelSpan = (
  source: LoadedSource,
  label: string,
): RuntimePresentationSourceSpan => {
  const start = source.lines.findIndex((line) =>
    labelPattern(label).test(normalizeAsmLine(line)),
  );
  if (start < 0) {
    throw new Error(
      `Runtime presentation control target ${label} is missing from ${source.file}`,
    );
  }
  let end = source.lines.length;
  for (let index = start + 1; index < source.lines.length; index += 1) {
    const match = normalizeAsmLine(source.lines[index]).match(
      /^([A-Za-z_][A-Za-z0-9_@.]*)::?$/,
    );
    if (match && !match[1].startsWith(".")) {
      end = index;
      break;
    }
  }
  return { file: source.file, start_line: start + 1, end_line: end };
};

const findTokenSpan = (
  source: LoadedSource,
  token: string,
): RuntimePresentationSourceSpan => {
  const line = source.lines.findIndex((candidate) => candidate.includes(token));
  if (line < 0) {
    throw new Error(
      `Required runtime presentation source token ${JSON.stringify(token)} is missing from ${source.file}`,
    );
  }
  return { file: source.file, start_line: line + 1, end_line: line + 1 };
};

const resourceKind = (
  resourcePath: string,
): "tiles" | "tilemap" | "attrmap" | "palette" => {
  if (resourcePath.includes(".tilemap")) return "tilemap";
  if (resourcePath.includes(".attrmap")) return "attrmap";
  if (/\.(?:pal|gbcpal)$/.test(resourcePath)) return "palette";
  return "tiles";
};

const exactKeys = (
  value: unknown,
  allowed: readonly string[],
  context: string,
): Record<string, unknown> => {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    throw new Error(`${context} must be an object`);
  }
  const record = value as Record<string, unknown>;
  for (const key of Object.keys(record)) {
    if (!allowed.includes(key)) {
      throw new Error(`${context} has unknown field ${key}`);
    }
  }
  return record;
};

const assertSpan = (value: unknown, context: string): void => {
  const span = exactKeys(value, ["file", "start_line", "end_line"], context);
  if (
    typeof span.file !== "string" ||
    !Number.isInteger(span.start_line) ||
    !Number.isInteger(span.end_line) ||
    (span.start_line as number) <= 0 ||
    (span.end_line as number) < (span.start_line as number)
  ) {
    throw new Error(`${context} is not an exact positive source span`);
  }
};

const assertDirectCallInvocation = (
  value: unknown,
  context: string,
): Record<string, unknown> => {
  const invocation = exactKeys(
    value,
    ["call_form", "target", "stack_effect", "register_result", "source_span"],
    context,
  );
  if (
    invocation.call_form !== "call" ||
    typeof invocation.target !== "string" ||
    !invocation.target ||
    invocation.stack_effect !== "push_return_address_then_ret"
  ) {
    throw new Error(`${context} has an incomplete direct-call contract`);
  }
  const result = exactKeys(
    invocation.register_result,
    ["a", "bc", "de", "hl", "flags"],
    `${context} register_result`,
  );
  if (
    !["number", "string"].includes(typeof result.a) ||
    typeof result.bc !== "string" ||
    typeof result.de !== "string" ||
    typeof result.hl !== "string"
  ) {
    throw new Error(`${context} has an incomplete register result`);
  }
  if (result.flags !== "unchanged") {
    const flags = exactKeys(
      result.flags,
      ["zero", "subtract", "half_carry", "carry"],
      `${context} flags`,
    );
    if (
      Object.values(flags).some(
        (flag) => typeof flag !== "boolean" && typeof flag !== "string",
      )
    ) {
      throw new Error(`${context} has an incomplete flags result`);
    }
  }
  assertSpan(invocation.source_span, `${context} source_span`);
  return invocation;
};

const OPERATION_KEYS: Record<string, readonly string[]> = {
  display_state: [
    "op",
    "registers",
    "scanline_scroll",
    "tile_layers",
    "palettes",
    "oam",
    "source_span",
  ],
  display_transition: [
    "op",
    "frames",
    "register_steps",
    "scanline_scroll",
    "oam_translation",
    "source_span",
  ],
  set_local: ["op", "name", "value", "source_span"],
  add_local: ["op", "name", "delta", "minimum", "maximum", "source_span"],
  input_branch: [
    "op",
    "sample",
    "require_all",
    "require_any",
    "forbid_any",
    "target",
    "source_span",
  ],
  branch_result: [
    "op",
    "result",
    "equals",
    "target",
    "else_target",
    "source_span",
  ],
  jump: ["op", "target", "source_span"],
  wait_frames: [
    "op",
    "frames",
    "condition",
    "implementation_source_span",
    "invocation",
    "source_span",
  ],
  sprite_scheduler_step: [
    "op",
    "instances",
    "struct_slots",
    "callback_before_frame_update",
    "oam_cursor",
    "remaining_oam_clear",
    "caller_register_state",
    "before_host_call",
    "reset_source_span",
    "implementation_source_spans",
    "invocation",
    "source_span",
  ],
  play_audio: ["op", "audio", "source_span"],
  stop_audio: ["op", "audio", "source_span"],
  fade_audio: ["op", "audio", "frames", "source_span"],
  host_effect: ["op", "effect", "result", "args", "source_span"],
  fill_memory: [
    "op",
    "target",
    "target_end_exclusive",
    "byte_count",
    "value",
    "direction",
    "bank",
    "condition",
    "value_source_span",
    "destination_views",
    "destination_labels",
    "implementation_source_span",
    "layout_source_spans",
    "invocation",
    "source_span",
  ],
  write_memory_word: [
    "op",
    "target",
    "value",
    "byte_order",
    "condition",
    "source_span",
  ],
  write_memory_byte: [
    "op",
    "target",
    "value",
    "address_space",
    "condition",
    "value_source_span",
    "target_declaration_source_span",
    "target_section_source_span",
    "implementation_source_span",
    "invocation",
    "source_span",
  ],
  increment_memory_byte: [
    "op",
    "target",
    "address_space",
    "delta",
    "wrap",
    "target_declaration_source_span",
    "target_section_source_span",
    "implementation_source_span",
    "invocation",
    "source_span",
  ],
  palette_transfer_request: [
    "op",
    "condition",
    "request",
    "background",
    "objects",
    "schedule",
    "source_span",
    "implementation_source_spans",
  ],
  show_text: ["op", "text", "source_span"],
  menu: [
    "op",
    "id",
    "items",
    "default_index",
    "wrap",
    "allow_cancel",
    "input_sample",
    "result",
    "source_span",
  ],
  call_subprogram: ["op", "program", "result", "source_span"],
  return: ["op", "source_span"],
};

export function assertRuntimePresentationProgram(
  value: unknown,
): asserts value is RuntimePresentationProgram {
  const program = exactKeys(
    value,
    [
      "schema_version",
      "entrypoints",
      "blocks",
      "resources",
      "audio",
      "text",
      "host_effects",
      "subprograms",
    ],
    "RuntimePresentationProgram",
  );
  if (program.schema_version !== 1) {
    throw new Error(
      "RuntimePresentationProgram schema_version must be exactly 1",
    );
  }
  const entrypoints = exactKeys(
    program.entrypoints,
    RUNTIME_PRESENTATION_ENTRYPOINTS,
    "RuntimePresentationProgram entrypoints",
  );
  for (const entrypoint of RUNTIME_PRESENTATION_ENTRYPOINTS) {
    if (typeof entrypoints[entrypoint] !== "string") {
      throw new Error(
        `RuntimePresentationProgram entrypoint ${entrypoint} is missing`,
      );
    }
  }
  const blocks = exactKeys(
    program.blocks,
    Object.keys(program.blocks as object),
    "RuntimePresentationProgram blocks",
  );
  const resources = program.resources;
  const audio = program.audio;
  const text = program.text;
  const hostEffects = program.host_effects;
  const subprograms = program.subprograms;
  if (
    ![resources, audio, text, hostEffects, subprograms].every(Array.isArray)
  ) {
    throw new Error("RuntimePresentationProgram catalogs must be arrays");
  }

  const resourcePaths = new Set<string>();
  for (const [index, candidate] of (resources as unknown[]).entries()) {
    const resource = exactKeys(
      candidate,
      ["path", "kind", "source_span"],
      `resource ${index}`,
    );
    if (typeof resource.path !== "string" || resourcePaths.has(resource.path)) {
      throw new Error(
        `Runtime presentation resource ${String(resource.path)} is missing or duplicated`,
      );
    }
    resourcePaths.add(resource.path);
    assertSpan(resource.source_span, `resource ${resource.path} source_span`);
  }
  const audioIds = new Set<string>();
  for (const [index, candidate] of (audio as unknown[]).entries()) {
    const reference = exactKeys(
      candidate,
      ["id", "kind", "source_span"],
      `audio ${index}`,
    );
    if (typeof reference.id !== "string" || audioIds.has(reference.id)) {
      throw new Error(
        `Runtime presentation audio ${String(reference.id)} is missing or duplicated`,
      );
    }
    audioIds.add(reference.id);
    assertSpan(reference.source_span, `audio ${reference.id} source_span`);
  }
  const textIds = new Set<string>();
  for (const [index, candidate] of (text as unknown[]).entries()) {
    const reference = exactKeys(
      candidate,
      ["id", "source_span"],
      `text ${index}`,
    );
    if (typeof reference.id !== "string" || textIds.has(reference.id)) {
      throw new Error(
        `Runtime presentation text ${String(reference.id)} is missing or duplicated`,
      );
    }
    textIds.add(reference.id);
    assertSpan(reference.source_span, `text ${reference.id} source_span`);
  }
  const hostEffectIds = new Set<string>();
  const hostEffectResults = new Map<string, string>();
  for (const [index, candidate] of (hostEffects as unknown[]).entries()) {
    const effect = exactKeys(
      candidate,
      [
        "id",
        "call_target",
        "accepted_call_forms",
        "result",
        "validity_checks",
        "state_deltas",
        "required_consumer",
        "source_span",
        "implementation_source_spans",
      ],
      `host effect ${index}`,
    );
    if (
      typeof effect.id !== "string" ||
      !effect.id ||
      hostEffectIds.has(effect.id)
    ) {
      throw new Error(
        `Runtime presentation host effect ${String(effect.id)} is missing or duplicated`,
      );
    }
    hostEffectIds.add(effect.id);
    if (typeof effect.call_target !== "string" || !effect.call_target) {
      throw new Error(`Host effect ${effect.id} call_target is missing`);
    }
    if (
      !Array.isArray(effect.accepted_call_forms) ||
      effect.accepted_call_forms.length === 0 ||
      !(effect.accepted_call_forms as unknown[]).every((form) =>
        ["call", "callfar", "farcall", "jump", "restart"].includes(
          String(form),
        ),
      )
    ) {
      throw new Error(
        `Host effect ${effect.id} accepted_call_forms must be exact typed call forms`,
      );
    }
    const result = exactKeys(
      effect.result,
      ["name", "type", "domain"],
      `host effect ${effect.id} result`,
    );
    if (
      typeof result.name !== "string" ||
      !result.name ||
      result.type !== "enum" ||
      !Array.isArray(result.domain) ||
      result.domain.length === 0
    ) {
      throw new Error(`Host effect ${effect.id} result enum is incomplete`);
    }
    hostEffectResults.set(effect.id, result.name);
    const resultIds = new Set<string>();
    const resultValues = new Set<number>();
    for (const [resultIndex, candidateResult] of (
      result.domain as unknown[]
    ).entries()) {
      const resultCase = exactKeys(
        candidateResult,
        ["id", "value", "conditions"],
        `host effect ${effect.id} result case ${resultIndex}`,
      );
      if (
        typeof resultCase.id !== "string" ||
        !resultCase.id ||
        !Number.isInteger(resultCase.value) ||
        resultIds.has(resultCase.id) ||
        resultValues.has(resultCase.value as number) ||
        !Array.isArray(resultCase.conditions) ||
        resultCase.conditions.length === 0
      ) {
        throw new Error(
          `Host effect ${effect.id} result domain has a missing or duplicate case`,
        );
      }
      resultIds.add(resultCase.id);
      resultValues.add(resultCase.value as number);
      for (const [conditionIndex, candidateCondition] of (
        resultCase.conditions as unknown[]
      ).entries()) {
        const condition = exactKeys(
          candidateCondition,
          ["source", "valid"],
          `host effect ${effect.id} result ${resultCase.id} condition ${conditionIndex}`,
        );
        if (
          typeof condition.source !== "string" ||
          !condition.source ||
          typeof condition.valid !== "boolean"
        ) {
          throw new Error(
            `Host effect ${effect.id} result ${resultCase.id} has an invalid condition`,
          );
        }
      }
    }
    if (
      !Array.isArray(effect.validity_checks) ||
      effect.validity_checks.length === 0
    ) {
      throw new Error(`Host effect ${effect.id} validity_checks are missing`);
    }
    for (const [checkIndex, candidateCheck] of (
      effect.validity_checks as unknown[]
    ).entries()) {
      const check = exactKeys(
        candidateCheck,
        ["source", "fields", "source_span"],
        `host effect ${effect.id} validity check ${checkIndex}`,
      );
      if (
        typeof check.source !== "string" ||
        !check.source ||
        !Array.isArray(check.fields) ||
        check.fields.length === 0
      ) {
        throw new Error(
          `Host effect ${effect.id} validity check ${checkIndex} is incomplete`,
        );
      }
      for (const [fieldIndex, candidateField] of (
        check.fields as unknown[]
      ).entries()) {
        const field = exactKeys(
          candidateField,
          ["name", "equals"],
          `host effect ${effect.id} validity field ${fieldIndex}`,
        );
        if (
          typeof field.name !== "string" ||
          !field.name ||
          !Number.isInteger(field.equals)
        ) {
          throw new Error(
            `Host effect ${effect.id} validity field ${fieldIndex} is incomplete`,
          );
        }
      }
      assertSpan(
        check.source_span,
        `host effect ${effect.id} validity check ${checkIndex} source_span`,
      );
    }
    if (
      !Array.isArray(effect.state_deltas) ||
      effect.state_deltas.length !== resultIds.size
    ) {
      throw new Error(
        `Host effect ${effect.id} state_deltas must cover every result exactly once`,
      );
    }
    const deltaResults = new Set<string>();
    for (const [deltaIndex, candidateDelta] of (
      effect.state_deltas as unknown[]
    ).entries()) {
      const delta = exactKeys(
        candidateDelta,
        ["when", "writes"],
        `host effect ${effect.id} state delta ${deltaIndex}`,
      );
      if (
        typeof delta.when !== "string" ||
        !resultIds.has(delta.when) ||
        deltaResults.has(delta.when) ||
        !Array.isArray(delta.writes) ||
        delta.writes.length === 0
      ) {
        throw new Error(
          `Host effect ${effect.id} state delta ${deltaIndex} does not exactly cover a result`,
        );
      }
      deltaResults.add(delta.when);
      for (const [writeIndex, candidateWrite] of (
        delta.writes as unknown[]
      ).entries()) {
        const rawWrite = candidateWrite as Record<string, unknown>;
        const kind = String(rawWrite?.kind ?? "<missing>");
        const keysByKind: Record<string, readonly string[]> = {
          copy_bytes: [
            "kind",
            "source",
            "target",
            "byte_count",
            "fields",
            "values",
            "source_span",
          ],
          copy_byte: ["kind", "source", "target", "source_span"],
          constant_byte: ["kind", "target", "value", "source_span"],
          zero_bytes: ["kind", "targets", "source_span"],
          persist_rtc: [
            "kind",
            "sources",
            "targets",
            "clears_halt",
            "source_span",
          ],
        };
        const allowed = keysByKind[kind];
        if (!allowed) {
          throw new Error(
            `Host effect ${effect.id} state delta ${delta.when} has unsupported write ${kind}`,
          );
        }
        const write = exactKeys(
          rawWrite,
          allowed,
          `host effect ${effect.id} state delta ${delta.when} write ${writeIndex}`,
        );
        if (
          ["copy_bytes", "copy_byte"].includes(kind) &&
          (typeof write.source !== "string" ||
            !write.source ||
            typeof write.target !== "string" ||
            !write.target)
        ) {
          throw new Error(
            `Host effect ${effect.id} ${kind} write is missing its source or target`,
          );
        }
        if (
          kind === "copy_bytes" &&
          (!Number.isInteger(write.byte_count) ||
            (write.byte_count as number) <= 0 ||
            !Array.isArray(write.fields) ||
            write.fields.length !== write.byte_count ||
            (write.values !== undefined &&
              (!Array.isArray(write.values) ||
                write.values.length !== write.byte_count)))
        ) {
          throw new Error(
            `Host effect ${effect.id} copy_bytes write does not exactly describe every byte`,
          );
        }
        if (
          kind === "constant_byte" &&
          (typeof write.target !== "string" ||
            !write.target ||
            !Number.isInteger(write.value) ||
            (write.value as number) < 0 ||
            (write.value as number) > 0xff)
        ) {
          throw new Error(
            `Host effect ${effect.id} constant_byte write is invalid`,
          );
        }
        if (
          kind === "zero_bytes" &&
          (!Array.isArray(write.targets) || write.targets.length === 0)
        ) {
          throw new Error(
            `Host effect ${effect.id} zero_bytes write has no targets`,
          );
        }
        if (
          kind === "persist_rtc" &&
          (!Array.isArray(write.sources) ||
            !Array.isArray(write.targets) ||
            write.sources.length === 0 ||
            write.sources.length !== write.targets.length ||
            write.clears_halt !== true)
        ) {
          throw new Error(
            `Host effect ${effect.id} persist_rtc write is incomplete`,
          );
        }
        assertSpan(
          write.source_span,
          `host effect ${effect.id} state delta ${delta.when} write ${writeIndex} source_span`,
        );
      }
    }
    const consumer = exactKeys(
      effect.required_consumer,
      ["id", "required"],
      `host effect ${effect.id} required_consumer`,
    );
    if (
      typeof consumer.id !== "string" ||
      !consumer.id ||
      consumer.required !== true
    ) {
      throw new Error(
        `Host effect ${effect.id} must name an exact required runtime consumer`,
      );
    }
    assertSpan(effect.source_span, `host effect ${effect.id} source_span`);
    if (
      !Array.isArray(effect.implementation_source_spans) ||
      effect.implementation_source_spans.length === 0
    ) {
      throw new Error(
        `Host effect ${effect.id} implementation_source_spans are missing`,
      );
    }
    for (const [spanIndex, span] of (
      effect.implementation_source_spans as unknown[]
    ).entries()) {
      assertSpan(
        span,
        `host effect ${effect.id} implementation source span ${spanIndex}`,
      );
    }
  }
  const subprogramIds = new Set<string>();
  for (const [index, candidate] of (subprograms as unknown[]).entries()) {
    const subprogram = exactKeys(
      candidate,
      ["id", "source_entry", "source_span", "resources", "audio", "text"],
      `subprogram ${index}`,
    );
    if (typeof subprogram.id !== "string" || subprogramIds.has(subprogram.id)) {
      throw new Error(
        `Runtime presentation subprogram ${String(subprogram.id)} is missing or duplicated`,
      );
    }
    subprogramIds.add(subprogram.id);
    assertSpan(
      subprogram.source_span,
      `subprogram ${subprogram.id} source_span`,
    );
    for (const resource of subprogram.resources as string[]) {
      if (!resourcePaths.has(resource)) {
        throw new Error(
          `Subprogram ${subprogram.id} references missing resource ${resource}`,
        );
      }
    }
    for (const id of subprogram.audio as string[]) {
      if (!audioIds.has(id)) {
        throw new Error(
          `Subprogram ${subprogram.id} references missing audio ${id}`,
        );
      }
    }
    for (const id of subprogram.text as string[]) {
      if (!textIds.has(id)) {
        throw new Error(
          `Subprogram ${subprogram.id} references missing text ${id}`,
        );
      }
    }
  }

  for (const [entrypoint, target] of Object.entries(entrypoints)) {
    if (!Object.hasOwn(blocks, target as string)) {
      throw new Error(
        `Runtime presentation entrypoint ${entrypoint} targets missing block ${String(target)}`,
      );
    }
  }
  const referencedHostEffects = new Set<string>();
  for (const [blockId, candidate] of Object.entries(blocks)) {
    const block = exactKeys(
      candidate,
      ["source_span", "operations"],
      `block ${blockId}`,
    );
    assertSpan(block.source_span, `block ${blockId} source_span`);
    if (!Array.isArray(block.operations) || block.operations.length === 0) {
      throw new Error(
        `Runtime presentation block ${blockId} has no operations`,
      );
    }
    for (const [index, operationValue] of block.operations.entries()) {
      const raw = operationValue as Record<string, unknown>;
      const op = typeof raw?.op === "string" ? raw.op : "<missing>";
      const allowed = OPERATION_KEYS[op];
      if (!allowed) {
        throw new Error(
          `Runtime presentation block ${blockId} has unsupported operation ${op}`,
        );
      }
      const operation = exactKeys(
        raw,
        allowed,
        `block ${blockId} operation ${index}`,
      );
      assertSpan(
        operation.source_span,
        `block ${blockId} operation ${index} source_span`,
      );
      if (["jump", "input_branch", "branch_result"].includes(op)) {
        for (const targetKey of ["target", "else_target"] as const) {
          const target = operation[targetKey];
          if (
            target !== undefined &&
            (typeof target !== "string" || !Object.hasOwn(blocks, target))
          ) {
            throw new Error(
              `Runtime presentation operation targets missing block ${String(target)}`,
            );
          }
        }
      }
      if (
        op === "host_effect" &&
        !hostEffectIds.has(String(operation.effect))
      ) {
        throw new Error(
          `Runtime presentation operation references missing host effect ${String(operation.effect)}`,
        );
      }
      if (op === "host_effect") {
        const effectId = String(operation.effect);
        referencedHostEffects.add(effectId);
        if (
          operation.result !== hostEffectResults.get(effectId) ||
          !Array.isArray(operation.args)
        ) {
          throw new Error(
            `Runtime presentation operation for host effect ${effectId} has the wrong result slot or arguments`,
          );
        }
      }
      if (
        [
          "fill_memory",
          "write_memory_word",
          "write_memory_byte",
          "palette_transfer_request",
          "wait_frames",
        ].includes(op)
      ) {
        const condition = exactKeys(
          operation.condition,
          ["source", "predicate", "bit", "source_span"],
          `block ${blockId} ${op} condition`,
        );
        const predicate = String(condition.predicate);
        if (
          !["always", "zero", "nonzero", "bit_set", "bit_clear"].includes(
            predicate,
          )
        ) {
          throw new Error(
            `Runtime presentation block ${blockId} ${op} has an invalid memory condition`,
          );
        }
        if (predicate === "always") {
          if (
            condition.source !== null ||
            condition.bit !== undefined ||
            condition.source_span !== null
          ) {
            throw new Error(
              `Runtime presentation block ${blockId} ${op} unconditional memory condition is not exact`,
            );
          }
        } else {
          if (
            typeof condition.source !== "string" ||
            !condition.source ||
            condition.source_span === null ||
            condition.source_span === undefined
          ) {
            throw new Error(
              `Runtime presentation block ${blockId} ${op} conditional memory source is incomplete`,
            );
          }
          assertSpan(
            condition.source_span,
            `block ${blockId} ${op} condition source_span`,
          );
          if (predicate === "bit_set" || predicate === "bit_clear") {
            const bit = exactKeys(
              condition.bit,
              ["symbol", "value"],
              `block ${blockId} ${op} condition bit`,
            );
            if (
              typeof bit.symbol !== "string" ||
              !bit.symbol ||
              !Number.isInteger(bit.value) ||
              (bit.value as number) < 0 ||
              (bit.value as number) > 7
            ) {
              throw new Error(
                `Runtime presentation block ${blockId} ${op} condition bit is invalid`,
              );
            }
          } else if (condition.bit !== undefined) {
            throw new Error(
              `Runtime presentation block ${blockId} ${op} non-bit condition has a bit operand`,
            );
          }
        }
      }
      if (op === "fill_memory") {
        const bank = exactKeys(
          operation.bank,
          ["select", "restore"],
          `block ${blockId} fill_memory bank`,
        );
        if (
          typeof operation.target !== "string" ||
          !operation.target ||
          !Number.isInteger(operation.byte_count) ||
          (operation.byte_count as number) <= 0 ||
          !Number.isInteger(operation.value) ||
          (operation.value as number) < 0 ||
          (operation.value as number) > 0xff ||
          !["ascending", "descending"].includes(String(operation.direction)) ||
          typeof bank.select !== "string" ||
          !bank.select ||
          typeof bank.restore !== "boolean"
        ) {
          throw new Error(
            `Runtime presentation block ${blockId} has an incomplete fill_memory operation`,
          );
        }
        if (operation.value_source_span !== undefined) {
          assertSpan(
            operation.value_source_span,
            `block ${blockId} fill_memory value_source_span`,
          );
        }
        const detailedFields = [
          operation.target_end_exclusive,
          operation.destination_views,
          operation.destination_labels,
          operation.implementation_source_span,
          operation.layout_source_spans,
          operation.invocation,
        ];
        if (detailedFields.some((field) => field !== undefined)) {
          if (
            typeof operation.target_end_exclusive !== "string" ||
            !operation.target_end_exclusive ||
            !Array.isArray(operation.destination_views) ||
            operation.destination_views.length === 0 ||
            !Array.isArray(operation.destination_labels) ||
            operation.destination_labels.length === 0 ||
            !Array.isArray(operation.layout_source_spans) ||
            operation.layout_source_spans.length === 0
          ) {
            throw new Error(
              `Runtime presentation block ${blockId} detailed fill_memory certificate is incomplete`,
            );
          }
          const viewIds = new Set<string>();
          for (const [viewIndex, candidateView] of (
            operation.destination_views as unknown[]
          ).entries()) {
            const view = exactKeys(
              candidateView,
              ["id", "byte_offset", "byte_count"],
              `block ${blockId} fill_memory destination view ${viewIndex}`,
            );
            if (
              typeof view.id !== "string" ||
              !view.id ||
              viewIds.has(view.id) ||
              !Number.isInteger(view.byte_offset) ||
              (view.byte_offset as number) < 0 ||
              !Number.isInteger(view.byte_count) ||
              (view.byte_count as number) <= 0 ||
              (view.byte_offset as number) + (view.byte_count as number) >
                (operation.byte_count as number)
            ) {
              throw new Error(
                `Runtime presentation block ${blockId} fill_memory destination view ${viewIndex} is invalid`,
              );
            }
            viewIds.add(view.id as string);
          }
          const destinationLabels = operation.destination_labels as unknown[];
          if (
            destinationLabels.some(
              (label) => typeof label !== "string" || !label,
            ) ||
            new Set(destinationLabels).size !== destinationLabels.length
          ) {
            throw new Error(
              `Runtime presentation block ${blockId} fill_memory destination labels are missing or duplicated`,
            );
          }
          assertSpan(
            operation.implementation_source_span,
            `block ${blockId} fill_memory implementation_source_span`,
          );
          for (const [spanIndex, span] of (
            operation.layout_source_spans as unknown[]
          ).entries()) {
            assertSpan(
              span,
              `block ${blockId} fill_memory layout source span ${spanIndex}`,
            );
          }
          const invocation = exactKeys(
            operation.invocation,
            [
              "call_form",
              "target",
              "target_bank",
              "restores_rom_bank",
              "preserves_callee_bc",
              "scratch_writes",
              "register_result",
              "source_span",
              "macro_source_span",
              "implementation_source_spans",
            ],
            `block ${blockId} fill_memory invocation`,
          );
          if (
            invocation.call_form !== "farcall" ||
            typeof invocation.target !== "string" ||
            !invocation.target ||
            invocation.target_bank !== `BANK(${invocation.target})` ||
            invocation.restores_rom_bank !== true ||
            invocation.preserves_callee_bc !== true ||
            !Array.isArray(invocation.scratch_writes) ||
            invocation.scratch_writes.length === 0 ||
            !Array.isArray(invocation.implementation_source_spans) ||
            invocation.implementation_source_spans.length === 0
          ) {
            throw new Error(
              `Runtime presentation block ${blockId} fill_memory farcall invocation is incomplete`,
            );
          }
          for (const [writeIndex, candidateWrite] of (
            invocation.scratch_writes as unknown[]
          ).entries()) {
            const write = exactKeys(
              candidateWrite,
              ["target", "value"],
              `block ${blockId} fill_memory farcall scratch write ${writeIndex}`,
            );
            if (
              typeof write.target !== "string" ||
              !write.target ||
              !(
                typeof write.value === "string" ||
                (Array.isArray(write.value) &&
                  write.value.every((byte) => Number.isInteger(byte)))
              )
            ) {
              throw new Error(
                `Runtime presentation block ${blockId} fill_memory farcall scratch write ${writeIndex} is invalid`,
              );
            }
          }
          const registerResult = exactKeys(
            invocation.register_result,
            ["a", "bc", "hl", "de", "flags"],
            `block ${blockId} fill_memory farcall register result`,
          );
          const flags = exactKeys(
            registerResult.flags,
            ["zero", "subtract", "half_carry", "carry"],
            `block ${blockId} fill_memory farcall flags`,
          );
          if (
            !Number.isInteger(registerResult.a) ||
            !Number.isInteger(registerResult.bc) ||
            typeof registerResult.hl !== "string" ||
            typeof registerResult.de !== "string" ||
            Object.values(flags).some((flag) => typeof flag !== "boolean")
          ) {
            throw new Error(
              `Runtime presentation block ${blockId} fill_memory farcall register result is incomplete`,
            );
          }
          assertSpan(
            invocation.source_span,
            `block ${blockId} fill_memory farcall source_span`,
          );
          assertSpan(
            invocation.macro_source_span,
            `block ${blockId} fill_memory farcall macro_source_span`,
          );
          for (const [spanIndex, span] of (
            invocation.implementation_source_spans as unknown[]
          ).entries()) {
            assertSpan(
              span,
              `block ${blockId} fill_memory farcall implementation source span ${spanIndex}`,
            );
          }
        }
      }
      if (
        op === "write_memory_word" &&
        (typeof operation.target !== "string" ||
          !operation.target ||
          typeof operation.value !== "string" ||
          !operation.value ||
          operation.byte_order !== "little_endian")
      ) {
        throw new Error(
          `Runtime presentation block ${blockId} has an incomplete write_memory_word operation`,
        );
      }
      if (op === "write_memory_byte") {
        const numericValue =
          Number.isInteger(operation.value) &&
          (operation.value as number) >= 0 &&
          (operation.value as number) <= 0xff;
        const symbolicValue =
          typeof operation.value === "string" &&
          /^(?:HIGH|LOW)\([A-Za-z_.][A-Za-z0-9_.@]*\)$/.test(operation.value);
        if (
          typeof operation.target !== "string" ||
          !operation.target ||
          (!numericValue && !symbolicValue) ||
          !["hram", "wram", "hardware_register"].includes(
            String(operation.address_space),
          ) ||
          (symbolicValue && operation.value_source_span === undefined)
        ) {
          throw new Error(
            `Runtime presentation block ${blockId} has an incomplete write_memory_byte operation`,
          );
        }
        if (operation.value_source_span !== undefined) {
          assertSpan(
            operation.value_source_span,
            `block ${blockId} write_memory_byte value_source_span`,
          );
        }
        if (operation.target_declaration_source_span !== undefined) {
          assertSpan(
            operation.target_declaration_source_span,
            `block ${blockId} write_memory_byte target declaration`,
          );
        }
        if (operation.target_section_source_span !== undefined) {
          assertSpan(
            operation.target_section_source_span,
            `block ${blockId} write_memory_byte target section`,
          );
        }
        if (
          operation.implementation_source_span !== undefined ||
          operation.invocation !== undefined
        ) {
          assertSpan(
            operation.implementation_source_span,
            `block ${blockId} write_memory_byte implementation_source_span`,
          );
          assertDirectCallInvocation(
            operation.invocation,
            `block ${blockId} write_memory_byte invocation`,
          );
        }
      }
      if (op === "increment_memory_byte") {
        if (
          typeof operation.target !== "string" ||
          !operation.target ||
          operation.address_space !== "wram" ||
          operation.delta !== 1 ||
          operation.wrap !== "u8"
        ) {
          throw new Error(
            `Runtime presentation block ${blockId} has an incomplete increment_memory_byte operation`,
          );
        }
        assertSpan(
          operation.target_declaration_source_span,
          `block ${blockId} increment_memory_byte target declaration`,
        );
        assertSpan(
          operation.target_section_source_span,
          `block ${blockId} increment_memory_byte target section`,
        );
        assertSpan(
          operation.implementation_source_span,
          `block ${blockId} increment_memory_byte implementation_source_span`,
        );
        assertDirectCallInvocation(
          operation.invocation,
          `block ${blockId} increment_memory_byte invocation`,
        );
      }
      if (op === "palette_transfer_request") {
        const request = exactKeys(
          operation.request,
          ["target", "queued_value", "completion_value"],
          `block ${blockId} palette transfer request flag`,
        );
        if (
          typeof request.target !== "string" ||
          !request.target ||
          request.queued_value !== 1 ||
          request.completion_value !== 0 ||
          operation.schedule !== "vblank"
        ) {
          throw new Error(
            `Runtime presentation block ${blockId} has an incomplete palette transfer request`,
          );
        }
        for (const layerName of ["background", "objects"] as const) {
          const layer = exactKeys(
            operation[layerName],
            [
              "source",
              "byte_count",
              "target",
              "index_register",
              "data_register",
              "autoincrement",
            ],
            `block ${blockId} palette transfer ${layerName}`,
          );
          if (
            typeof layer.source !== "string" ||
            !layer.source ||
            !Number.isInteger(layer.byte_count) ||
            (layer.byte_count as number) <= 0 ||
            typeof layer.target !== "string" ||
            !layer.target ||
            typeof layer.index_register !== "string" ||
            !layer.index_register ||
            typeof layer.data_register !== "string" ||
            !layer.data_register ||
            layer.autoincrement !== true
          ) {
            throw new Error(
              `Runtime presentation block ${blockId} has an incomplete ${layerName} palette transfer`,
            );
          }
        }
        if (
          !Array.isArray(operation.implementation_source_spans) ||
          operation.implementation_source_spans.length === 0
        ) {
          throw new Error(
            `Runtime presentation block ${blockId} palette transfer has no implementation source spans`,
          );
        }
        for (const [spanIndex, span] of (
          operation.implementation_source_spans as unknown[]
        ).entries()) {
          assertSpan(
            span,
            `block ${blockId} palette transfer implementation source span ${spanIndex}`,
          );
        }
      }
      if (
        op === "wait_frames" &&
        (!Number.isInteger(operation.frames) ||
          (operation.frames as number) <= 0)
      ) {
        throw new Error(
          `Runtime presentation block ${blockId} has an invalid wait_frames operation`,
        );
      }
      if (
        ["play_audio", "stop_audio", "fade_audio"].includes(op) &&
        !audioIds.has(String(operation.audio))
      ) {
        throw new Error(
          `Runtime presentation operation references missing audio ${String(operation.audio)}`,
        );
      }
      if (op === "show_text" && !textIds.has(String(operation.text))) {
        throw new Error(
          `Runtime presentation operation references missing text ${String(operation.text)}`,
        );
      }
      if (
        op === "call_subprogram" &&
        !subprogramIds.has(String(operation.program))
      ) {
        throw new Error(
          `Runtime presentation operation references missing subprogram ${String(operation.program)}`,
        );
      }
      if (op === "display_state") {
        const display = exactKeys(
          operation,
          OPERATION_KEYS.display_state,
          `block ${blockId} display_state`,
        );
        for (const layer of display.tile_layers as Array<
          Record<string, unknown>
        >) {
          if (!resourcePaths.has(String(layer.resource))) {
            throw new Error(
              `Display state references missing resource ${String(layer.resource)}`,
            );
          }
        }
        for (const palette of display.palettes as string[]) {
          if (!resourcePaths.has(palette)) {
            throw new Error(
              `Display state references missing resource ${palette}`,
            );
          }
        }
      }
    }
  }
  for (const effectId of hostEffectIds) {
    if (!referencedHostEffects.has(effectId)) {
      throw new Error(
        `Runtime presentation host effect ${effectId} is registered but unused`,
      );
    }
  }
}
