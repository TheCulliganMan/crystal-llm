/**
 * This module defines Enums related to UI.
 */

/**
 * Enumerate the three text-speed presets from the original game.
 */
export enum TextSpeed {
  FAST = 'fast',
  MID = 'mid',
  SLOW = 'slow',
}

/**
 * Audio output configuration.
 */
export enum Sound {
  MONO = 'mono',
  STEREO = 'stereo',
}

export enum MenuAccount {
  ON = 'on',
  OFF = 'off',
}

/**
 * Mirrors the hardware menu return codes from `hMenuReturn`.
 */
export enum HMenuReturn {
  REOPEN = 'reopen',
  EXIT = 'exit',
  SCRIPT = 'script',
  ASM = 'asm',
  REDRAW = 'redraw',
}

/**
 * Printer darkness options used by the original hardware.
 */
export enum PrintOption {
  LIGHTEST = 0x00,
  LIGHTER = 0x20,
  NORMAL = 0x40,
  DARKER = 0x60,
  DARKEST = 0x7f,
}

export const orderedPrintOptions = (): PrintOption[] => {
  return [
    PrintOption.LIGHTEST,
    PrintOption.LIGHTER,
    PrintOption.NORMAL,
    PrintOption.DARKER,
    PrintOption.DARKEST,
  ];
};

/**
 * Identifier for the decorative frame skin used around text windows.
 */
export enum FrameType {
  FRAME_1 = 0,
  FRAME_2 = 1,
  FRAME_3 = 2,
  FRAME_4 = 3,
  FRAME_5 = 4,
  FRAME_6 = 5,
  FRAME_7 = 6,
  FRAME_8 = 7,
}

export const orderedFrameTypes = (): FrameType[] => {
  return [
    FrameType.FRAME_1,
    FrameType.FRAME_2,
    FrameType.FRAME_3,
    FrameType.FRAME_4,
    FrameType.FRAME_5,
    FrameType.FRAME_6,
    FrameType.FRAME_7,
    FrameType.FRAME_8,
  ];
};

export const frameTypeRenderId = (frameType: FrameType): number => {
  return frameType + 1;
};
