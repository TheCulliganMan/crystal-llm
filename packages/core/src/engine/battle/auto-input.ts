import { B_PAD_A, B_PAD_DOWN, B_PAD_RIGHT } from "@pokecrystal/core/input/controls";
import type { JoypadState } from "@pokecrystal/core/core/memory/hram";

export const NO_INPUT = 0x00;
export const AUTO_INPUT = 0xff;

type ScriptedStep = [number, number];

// ASM mapping:
// - engine/events/catch_tutorial_input.asm::DudeAutoInput_A
// - engine/events/catch_tutorial_input.asm::DudeAutoInput_RightA
// - engine/events/catch_tutorial_input.asm::DudeAutoInput_DownA
//
// The raw stream bytes are expressed in joypad-poll counts. PromptButton polling
// is effectively frame-based, but menu loops in the original engine consume input
// more aggressively. The TS battle menu runs one update per frame, so menu
// auto-input streams need an explicit poll multiplier to match the in-game pacing.
const MENU_AUTO_INPUT_POLLS_PER_FRAME = 16;

class ScriptedInput {
  public index = 0;
  public remaining = 0;
  public currentInput = 0;
  public active = true;

  constructor(public readonly steps: ScriptedStep[]) {}

  step(ticks = 1): number | null {
    if (!this.active) {
      return null;
    }

    const totalTicks = Math.max(1, Math.trunc(ticks));
    let remainingTicks = totalTicks;
    let aggregate = 0;

    while (remainingTicks > 0 && this.active) {
      if (this.remaining <= 0) {
        if (this.index >= this.steps.length) {
          this.active = false;
          break;
        }
        const [inputValue, duration] = this.steps[this.index];
        this.index += 1;
        if (duration === 0xff) {
          this.active = false;
          break;
        }
        this.currentInput = inputValue & 0xff;
        this.remaining = Math.max(0, Math.trunc(duration));
      }

      aggregate |= this.currentInput;
      let consume = Math.min(this.remaining, remainingTicks);
      if (consume === 0) {
        consume = 1;
      }
      this.remaining = Math.max(0, this.remaining - consume);
      remainingTicks -= consume;
    }

    if (aggregate === 0 && !this.active) {
      return null;
    }
    return aggregate;
  }
}

export class DudeAutoInputController {
  public idleInput: number = NO_INPUT;

  private activeStream: ScriptedInput | null = null;
  private activeStreamPollsPerFrame = 1;
  private idleStream: ScriptedInput | null = new ScriptedInput([[NO_INPUT, 0xff]]);
  private enabled = true;

  private static STREAM_A: ScriptedStep[] = [
    [NO_INPUT, 0x50],
    [B_PAD_A, 0x00],
    [NO_INPUT, 0xff],
  ];
  private static STREAM_RIGHT_A: ScriptedStep[] = [
    [NO_INPUT, 0x08],
    [B_PAD_RIGHT, 0x00],
    [NO_INPUT, 0x08],
    [B_PAD_A, 0x00],
    [NO_INPUT, 0xff],
  ];
  private static STREAM_DOWN_A: ScriptedStep[] = [
    [NO_INPUT, 0xfe],
    [NO_INPUT, 0xfe],
    [NO_INPUT, 0xfe],
    [NO_INPUT, 0xfe],
    [B_PAD_DOWN, 0x00],
    [NO_INPUT, 0xfe],
    [NO_INPUT, 0xfe],
    [NO_INPUT, 0xfe],
    [NO_INPUT, 0xfe],
    [B_PAD_A, 0x00],
    [NO_INPUT, 0xff],
  ];

  private load(steps: ScriptedStep[], pollsPerFrame = 1): void {
    this.activeStream = new ScriptedInput([...steps]);
    this.activeStreamPollsPerFrame = Math.max(1, Math.trunc(pollsPerFrame));
    this.enabled = true;
    if (!this.idleStream) {
      this.idleStream = new ScriptedInput([[NO_INPUT, 0xff]]);
    }
  }

  resetIdle(): void {
    this.activeStream = null;
    this.activeStreamPollsPerFrame = 1;
    if (this.idleStream) {
      this.idleStream = new ScriptedInput([...this.idleStream.steps]);
    } else {
      this.idleStream = new ScriptedInput([[NO_INPUT, 0xff]]);
    }
    this.enabled = true;
  }

  queueDownA(): void {
    this.load(DudeAutoInputController.STREAM_DOWN_A, MENU_AUTO_INPUT_POLLS_PER_FRAME);
  }

  queueRightA(): void {
    this.load(DudeAutoInputController.STREAM_RIGHT_A, MENU_AUTO_INPUT_POLLS_PER_FRAME);
  }

  queueA(): void {
    this.load(DudeAutoInputController.STREAM_A);
  }

  stop(): void {
    this.activeStream = null;
    this.activeStreamPollsPerFrame = 1;
    if (this.idleStream) {
      this.idleStream.active = false;
    }
    this.enabled = false;
  }

  step(joypad: JoypadState, ticks = 1): boolean {
    if (!this.enabled) {
      DudeAutoInputController.writeInput(joypad, this.idleInput);
      return false;
    }

    const stream = this.activeStream ?? this.idleStream;
    if (!stream) {
      DudeAutoInputController.writeInput(joypad, this.idleInput);
      return false;
    }

    const frameTicks = Math.max(1, Math.trunc(ticks));
    const pollsPerFrame =
      stream === this.activeStream ? this.activeStreamPollsPerFrame : 1;
    let value = stream.step(frameTicks * pollsPerFrame);
    if (!stream.active && stream === this.activeStream) {
      this.activeStream = null;
      this.activeStreamPollsPerFrame = 1;
    }
    if (value === null) {
      value = this.idleInput;
    }
    DudeAutoInputController.writeInput(joypad, value);
    return Boolean(this.activeStream && this.activeStream.active);
  }

  private static writeInput(joypad: JoypadState, value: number): void {
    joypad.hJoyPressed = value;
    joypad.hJoyDown = value;
    joypad.hJoyLast = value;
    joypad.hJoyReleased = 0;
    joypad.hJoypadPressed = value;
    joypad.hJoypadDown = value;
    joypad.hJoypadReleased = 0;
    joypad.hJoypadSum = value;
  }
}
