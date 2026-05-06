import path from "path";
import { z } from "zod";
import {
  InputEvent,
  TerminalInputAdapter,
} from "./adapters";
import {
  LoopInstruction,
  MacroDefinition,
  ScriptInstruction,
  ScriptInstructionType,
  ScriptProgram,
  ScriptTokenizer,
} from "./script-tokens";

export enum WaitMode {
  NONE = "none",
  FRAMES = "frames",
  PROMPT = "prompt",
  DIALOGUE_CLEAR = "dialogue_clear",
  NOT_BUSY = "not_busy",
}

const WaitMode_VALUES = new Set(Object.values(WaitMode));

export type ExecutionFrame = {
  program: ScriptProgram;
  pointer: number;
  loopRuntime?: LoopRuntime | null;
  name?: string | null;
  meta?: Record<string, unknown> | null;
};

export type LoopRuntime = {
  instruction: LoopInstruction;
  remaining: number;
};

export type CallFrame = {
  frame: ExecutionFrame;
  returnPointer: number;
};

const FrameStateSchema = z
  .object({
    pointer: z.number().int().optional(),
    name: z.string().nullable().optional(),
    type: z.string().optional(),
    meta: z.record(z.string(), z.unknown()).optional(),
    loop_remaining: z.number().int().optional(),
    loop_count: z.number().int().optional(),
    raw_body: z.array(z.unknown()).optional(),
  })
  .passthrough();

const CallStateSchema = z
  .object({
    frame_index: z.number().int().optional(),
    return_pointer: z.number().int().optional(),
  })
  .passthrough();

const ScriptStateSchema = z
  .object({
    version: z.literal(1),
    script_path: z.string().nullable().optional(),
    raw_script: z.array(z.unknown()).optional(),
    current_label: z.string().nullable().optional(),
    pending_wait_frames: z.number().int().nonnegative().optional(),
    wait_mode: z.string().optional(),
    prompt_pending: z.boolean().optional(),
    prompt_reason: z.string().nullable().optional(),
    dialogue_state: z.record(z.string(), z.unknown()).nullable().optional(),
    multi_choice_state: z.record(z.string(), z.unknown()).nullable().optional(),
    breakpoint_label: z.string().nullable().optional(),
    paused_on_breakpoint: z.boolean().optional(),
    autosave_counter: z.number().int().nullable().optional(),
    frames: z.array(FrameStateSchema).optional(),
    calls: z.array(CallStateSchema).optional(),
  })
  .passthrough();

type ScriptFrameState = z.infer<typeof FrameStateSchema>;
type ScriptCallState = z.infer<typeof CallStateSchema>;
type ScriptState = z.infer<typeof ScriptStateSchema>;

export class ScriptInputAdapter extends TerminalInputAdapter {
  private readonly rawScript: unknown[];
  private readonly basePath: string | null;
  private readonly tokenizer: ScriptTokenizer;
  private readonly program: ScriptProgram;
  private frameStack: ExecutionFrame[];
  private callStack: CallFrame[] = [];
  private waitMode: WaitMode = WaitMode.NONE;
  private pendingWaitFrames = 0;
  private promptPending = false;
  private promptReason: string | null = null;
  private dialogueState: Record<string, unknown> | null = null;
  private multiChoiceState: Record<string, unknown> | null = null;
  private currentLabel: string | null = null;
  private breakpointLabel: string | null;
  public pausedOnBreakpoint = false;

  constructor(
    script: unknown[],
    options?: {
      stdin?: NodeJS.ReadableStream | null;
      breakpointLabel?: string | null;
      basePath?: string | null;
      state?: ScriptState | null;
    }
  ) {
    super({ stdin: options?.stdin ?? null });
    this.rawScript = [...script];
    this.basePath = options?.basePath ?? null;
    const baseDir = this.basePath ? path.dirname(this.basePath) : null;
    this.tokenizer = new ScriptTokenizer({ baseDir });
    this.program = this.tokenizer.parse(this.rawScript);
    this.frameStack = [
      {
        program: this.program,
        pointer: 0,
        name: "root",
        meta: { type: "root" },
      },
    ];
    this.breakpointLabel = options?.breakpointLabel ?? null;
    if (options?.state) {
      this.restoreState(options.state);
    }
  }

  get current_label(): string | null {
    return this.currentLabel;
  }

  get remaining_tokens(): number {
    let total = 0;
    for (const frame of this.frameStack) {
      total += Math.max(0, frame.program.instructions.length - frame.pointer);
      const runtime = frame.loopRuntime;
      if (runtime) {
        total += Math.max(0, runtime.remaining - 1) * runtime.instruction.program.instructions.length;
      }
    }
    return total;
  }

  get waiting_reason(): string | null {
    if (this.waitMode === WaitMode.NONE) {
      return null;
    }
    if (this.waitMode === WaitMode.FRAMES) {
      return "frames";
    }
    if (this.waitMode === WaitMode.PROMPT) {
      return "prompt";
    }
    if (this.waitMode === WaitMode.NOT_BUSY) {
      return "not_busy";
    }
    return "dialogue_clear";
  }

  get prompt_reason(): string | null {
    return this.promptReason;
  }

  get dialogue_state(): Record<string, unknown> | null {
    return this.dialogueState;
  }

  get multi_choice_state(): Record<string, unknown> | null {
    return this.multiChoiceState;
  }

  get prompt_pending(): boolean {
    return this.promptPending;
  }

  get cursor_index(): number {
    const top = this.frameStack.length > 0 ? this.frameStack[this.frameStack.length - 1] : null;
    return top ? top.pointer : 0;
  }

  get pending_wait_frames(): number {
    return this.pendingWaitFrames;
  }

  snapshot(options?: { autosaveCounter?: number | null }): ScriptState {
    const frameIndexMap = new Map<ExecutionFrame, number>();
    this.frameStack.forEach((frame, idx) => frameIndexMap.set(frame, idx));
    const frames: ScriptFrameState[] = this.frameStack.map((frame) => this.serializeFrame(frame));
    const calls: ScriptCallState[] = this.callStack.map((call) => ({
      frame_index: frameIndexMap.get(call.frame) ?? -1,
      return_pointer: call.returnPointer,
    }));
    return {
      version: 1,
      script_path: this.basePath,
      raw_script: this.rawScript,
      current_label: this.currentLabel,
      pending_wait_frames: this.pendingWaitFrames,
      wait_mode: this.waitMode,
      prompt_pending: this.promptPending,
      prompt_reason: this.promptReason,
      dialogue_state: this.dialogueState,
      multi_choice_state: this.multiChoiceState,
      breakpoint_label: this.breakpointLabel,
      paused_on_breakpoint: this.pausedOnBreakpoint,
      autosave_counter: options?.autosaveCounter ?? null,
      frames,
      calls,
    };
  }

  restoreState(state: ScriptState): void {
    const parsed = ScriptStateSchema.parse(state);
    const rawScript = parsed.raw_script ?? this.rawScript;
    if (JSON.stringify(rawScript) !== JSON.stringify(this.rawScript)) {
      throw new Error("Script state does not match loaded script.");
    }
    this.currentLabel = parsed.current_label ?? null;
    this.pendingWaitFrames = parsed.pending_wait_frames ?? 0;
    const waitModeValue = parsed.wait_mode ?? WaitMode.NONE;
    this.waitMode = WaitMode_VALUES.has(waitModeValue as WaitMode)
      ? (waitModeValue as WaitMode)
      : WaitMode.NONE;
    this.promptPending = parsed.prompt_pending ?? false;
    this.promptReason = parsed.prompt_reason ?? null;
    this.dialogueState = (parsed.dialogue_state as Record<string, unknown> | null) ?? null;
    this.multiChoiceState = (parsed.multi_choice_state as Record<string, unknown> | null) ?? null;
    this.breakpointLabel = parsed.breakpoint_label ?? null;
    this.pausedOnBreakpoint = parsed.paused_on_breakpoint ?? false;

    const frameStates = parsed.frames ?? [];
    this.frameStack = frameStates.map((frameState) => this.frameFromState(frameState));
    const callStates: ScriptCallState[] = parsed.calls ?? [];
    this.callStack = [];
    for (const call of callStates) {
      const frameIdx = Number(call.frame_index ?? -1);
      if (frameIdx < 0 || frameIdx >= this.frameStack.length) {
        throw new Error("Invalid call frame index in script state.");
      }
      this.callStack.push({
        frame: this.frameStack[frameIdx],
        returnPointer: Number(call.return_pointer ?? 0),
      });
    }
    if (this.frameStack.length === 0) {
      this.frameStack = [
        {
          program: this.program,
          pointer: 0,
          name: "root",
          meta: { type: "root" },
        },
      ];
    }
  }

  updatePromptState(
    promptPending: boolean,
    promptReason: string | null = null,
    options?: {
      dialogueState?: Record<string, unknown> | null;
      multiChoiceState?: Record<string, unknown> | null;
    }
  ): void {
    this.promptPending = promptPending;
    this.promptReason = promptReason;
    this.dialogueState = options?.dialogueState ?? null;
    this.multiChoiceState = options?.multiChoiceState ?? null;
    if (this.waitMode === WaitMode.PROMPT && promptPending) {
      this.waitMode = WaitMode.NONE;
    }
    if (this.waitMode === WaitMode.DIALOGUE_CLEAR && !promptPending) {
      this.waitMode = WaitMode.NONE;
    }
    if (this.waitMode === WaitMode.NOT_BUSY && !promptPending) {
      this.waitMode = WaitMode.NONE;
    }
  }

  resumeBreakpoint(): void {
    this.pausedOnBreakpoint = false;
  }

  poll(): InputEvent[] {
    const manualEvents = this.consumeManualTokens();
    if (this.pausedOnBreakpoint) {
      return manualEvents;
    }
    if (this.waitMode === WaitMode.FRAMES) {
      if (this.pendingWaitFrames > 0) {
        this.pendingWaitFrames -= 1;
      }
      if (this.pendingWaitFrames <= 0) {
        this.waitMode = WaitMode.NONE;
      }
      return manualEvents;
    }
    if (this.waitMode === WaitMode.PROMPT) {
      if (!this.promptPending) {
        return manualEvents;
      }
      this.waitMode = WaitMode.NONE;
    } else if (this.waitMode === WaitMode.DIALOGUE_CLEAR || this.waitMode === WaitMode.NOT_BUSY) {
      if (this.promptPending) {
        return manualEvents;
      }
      this.waitMode = WaitMode.NONE;
    }

    const events: InputEvent[] = [];
    while (events.length === 0) {
      const nextInstruction = this.nextInstruction();
      if (!nextInstruction) {
        break;
      }
      const [instruction, frame] = nextInstruction;
      const outcome = this.handleInstruction(instruction, frame);
      if (outcome === "pause" || outcome === "wait") {
        break;
      }
      if (Array.isArray(outcome)) {
        events.push(...this.tokensToEvents(outcome));
      }
    }
    return events.concat(manualEvents);
  }

  private consumeManualTokens(): InputEvent[] {
    const events: InputEvent[] = [];
    for (const token of this.readTokens()) {
      events.push(...this.eventsForToken(token, "manual"));
    }
    return events;
  }

  private nextInstruction(): [ScriptInstruction, ExecutionFrame] | null {
    while (this.frameStack.length > 0) {
      const frame = this.frameStack[this.frameStack.length - 1];
      if (frame.pointer >= frame.program.instructions.length) {
        this.popFrame();
        continue;
      }
      const instruction = frame.program.instructions[frame.pointer];
      frame.pointer += 1;
      return [instruction, frame];
    }
    return null;
  }

  private handleInstruction(
    instruction: ScriptInstruction,
    frame: ExecutionFrame
  ): string | string[] | null {
    const kind = instruction.kind;
    if (kind === ScriptInstructionType.EMIT) {
      const tokens = Array.isArray(instruction.value) ? instruction.value : [];
      return tokens.map((token) => String(token));
    }
    if (kind === ScriptInstructionType.WAIT_FRAMES) {
      this.pendingWaitFrames = Math.max(0, Number(instruction.value ?? 0));
      this.waitMode = this.pendingWaitFrames > 0 ? WaitMode.FRAMES : WaitMode.NONE;
      return "wait";
    }
    if (kind === ScriptInstructionType.WAIT_PROMPT) {
      this.waitMode = WaitMode.PROMPT;
      return "wait";
    }
    if (kind === ScriptInstructionType.WAIT_DIALOGUE_CLEAR) {
      this.waitMode = WaitMode.DIALOGUE_CLEAR;
      return "wait";
    }
    if (kind === ScriptInstructionType.WAIT_NOT_BUSY) {
      this.waitMode = WaitMode.NOT_BUSY;
      return "wait";
    }
    if (kind === ScriptInstructionType.LABEL) {
      this.currentLabel = instruction.value ? String(instruction.value) : null;
      if (this.breakpointLabel && this.currentLabel === this.breakpointLabel) {
        this.pausedOnBreakpoint = true;
        return "pause";
      }
      return null;
    }
    if (kind === ScriptInstructionType.GOTO) {
      this.jumpToLabel(frame, String(instruction.value));
      return null;
    }
    if (kind === ScriptInstructionType.CALL) {
      const target = String(instruction.value);
      this.callStack.push({ frame, returnPointer: frame.pointer });
      this.jumpToLabel(frame, target);
      return null;
    }
    if (kind === ScriptInstructionType.RETURN) {
      this.handleReturn(frame);
      return null;
    }
    if (kind === ScriptInstructionType.MACRO_CALL) {
      const value = instruction.value as [MacroDefinition, unknown, number];
      const [macro, args, depthRemaining] = value;
      if (!(macro instanceof MacroDefinition)) {
        throw new Error("macro_call payload missing MacroDefinition.");
      }
      const macroProgram = macro.expand(args as string[] | Record<string, unknown>, this.tokenizer, {
        maxDepth: Math.max(1, depthRemaining) - 1,
      });
      this.frameStack.push({
        program: macroProgram,
        pointer: 0,
        name: `macro:${macro.name}`,
        meta: { type: "macro", name: macro.name, args: Array.isArray(args) ? [...args] : args },
      });
      return null;
    }
    if (kind === ScriptInstructionType.LOOP) {
      const loopInstruction = instruction.value as LoopInstruction;
      if (!(loopInstruction instanceof LoopInstruction)) {
        throw new Error("Invalid loop instruction payload.");
      }
      if (loopInstruction.count <= 0) {
        return null;
      }
      const runtime: LoopRuntime = {
        instruction: loopInstruction,
        remaining: loopInstruction.count,
      };
      this.frameStack.push({
        program: loopInstruction.program,
        pointer: 0,
        loopRuntime: runtime,
        name: "loop",
        meta: {
          type: "loop",
          count: loopInstruction.count,
          raw_body: loopInstruction.rawBody ?? [],
        },
      });
      return null;
    }
    throw new Error(`Unknown instruction kind '${instruction.kind}'`);
  }

  private jumpToLabel(frame: ExecutionFrame, target: string): void {
    if (!(target in frame.program.labels)) {
      throw new Error(`Label '${target}' is not defined.`);
    }
    frame.pointer = frame.program.labels[target];
  }

  private handleReturn(frame: ExecutionFrame): void {
    for (let idx = this.callStack.length - 1; idx >= 0; idx -= 1) {
      const callFrame = this.callStack[idx];
      if (callFrame.frame === frame) {
        frame.pointer = callFrame.returnPointer;
        this.callStack.splice(idx, 1);
        return;
      }
    }
    if (this.frameStack.length > 1) {
      this.frameStack.pop();
      return;
    }
    throw new Error("Return invoked without an active call frame.");
  }

  private popFrame(): void {
    const frame = this.frameStack.pop();
    if (!frame) {
      return;
    }
    const danglingCalls = this.callStack.filter((call) => call.frame === frame);
    if (danglingCalls.length > 0) {
      throw new Error("Unbalanced call stack; missing return before frame end.");
    }
    this.callStack = this.callStack.filter((call) => call.frame !== frame);
    const runtime = frame.loopRuntime;
    if (runtime) {
      runtime.remaining -= 1;
      if (runtime.remaining > 0) {
        this.frameStack.push({
          program: runtime.instruction.program,
          pointer: 0,
          loopRuntime: runtime,
          name: frame.name,
          meta: frame.meta,
        });
      }
    }
  }

  private tokensToEvents(tokens: string[]): InputEvent[] {
    const events: InputEvent[] = [];
    for (const token of tokens) {
      events.push(...this.eventsForToken(token, "script"));
    }
    return events;
  }

  private serializeFrame(frame: ExecutionFrame): ScriptFrameState {
    const runtime = frame.loopRuntime;
    const meta = frame.meta ?? {};
    const frameType = String(meta.type ?? (frame.name === "root" ? "root" : "frame"));
    const payload: ScriptFrameState = {
      pointer: frame.pointer,
      name: frame.name ?? null,
      type: frameType,
      meta,
    };
    if (runtime) {
      payload.loop_remaining = runtime.remaining;
      payload.loop_count = runtime.instruction.count;
      payload.raw_body = runtime.instruction.rawBody ?? [];
    }
    return payload;
  }

  private frameFromState(frameState: ScriptFrameState): ExecutionFrame {
    let frameType = String(frameState.type ?? "").toLowerCase();
    const meta = frameState.meta ?? {};
    if (!frameType) {
      frameType = String(meta.type ?? "").toLowerCase();
    }
    let frame: ExecutionFrame;
    if (frameType === "macro") {
      const macroName = meta.name ?? meta.macro;
      const rawArgs = Array.isArray(meta.args) ? meta.args : [];
      const stringArgs = rawArgs.map((value) => String(value));
      const macro = macroName ? this.tokenizer.macros[String(macroName)] : null;
      if (!macro) {
        throw new Error(`Macro '${String(macroName)}' missing during restore.`);
      }
      const program = macro.expand(stringArgs, this.tokenizer);
      frame = {
        program,
        pointer: Number(frameState.pointer ?? 0),
        name: frameState.name ? String(frameState.name) : null,
        meta: { type: "macro", name: macroName, args: stringArgs },
      };
    } else if (frameType === "loop") {
      const loopCountRaw = frameState.loop_count ?? meta.count ?? 0;
      const loopCount = Number(loopCountRaw ?? 0);
      const potentialBody = frameState.raw_body ?? meta.raw_body ?? [];
      const rawBody = Array.isArray(potentialBody) ? potentialBody : [];
      const loopProgram = this.tokenizer.parse(rawBody, { maxDepth: 4 });
      const loopInstruction = new LoopInstruction(loopProgram, loopCount, [...rawBody]);
      const remainingRaw = frameState.loop_remaining ?? loopCount;
      const remaining = Number(remainingRaw ?? loopCount);
      frame = {
        program: loopProgram,
        pointer: Number(frameState.pointer ?? 0),
        loopRuntime: { instruction: loopInstruction, remaining },
        name: frameState.name ? String(frameState.name) : null,
        meta: { type: "loop", count: loopCount, raw_body: rawBody },
      };
    } else {
      frame = {
        program: this.program,
        pointer: Number(frameState.pointer ?? 0),
        name: frameState.name ? String(frameState.name) : null,
        meta: { type: frameType || "root" },
      };
    }
    frame.pointer = Math.max(0, Math.min(frame.pointer, frame.program.instructions.length));
    return frame;
  }
}
