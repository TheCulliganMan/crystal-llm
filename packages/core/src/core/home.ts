import { GameState } from './state';
import { HardwareRNG } from '../engine/games/rng';
import { MoveName, PokemonType } from './enums';
import { LANDMARK_FAST_SHIP, LANDMARK_SPECIAL, Pokemon, Region, TMHM_MOVES } from './constants';
import { MAP_TO_LANDMARK, POKEGEAR_LANDMARKS, LandmarkEntry } from '@pokecrystal/assets/content/data/pokegear-landmarks';
import { getMapMetadataByGroup } from '../engine/world/maps';
import { GB_FRAME_DURATION_SECONDS } from './gb-timing';

const FRAME_DURATION = GB_FRAME_DURATION_SECONDS;

export function delayFrames(numFrames: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, numFrames * FRAME_DURATION * 1000));
}

export function clearJoypad(state: GameState): void {
  state.hram.joypad.hJoyPressed = 0;
  state.hram.joypad.hJoyDown = 0;
}

export function updateJoypad(state: GameState): void {
  const rng = new HardwareRNG(state);
  const realInput = state.hram.joypad.hJoypadDown ?? rng.nextByte();
  const lastFrame = state.hram.joypad.hJoyDown;
  const delta = lastFrame ^ realInput;

  state.hram.joypad.hJoyReleased = delta & lastFrame;
  state.hram.joypad.hJoyPressed = delta & realInput;
  state.hram.joypad.hJoyDown = realInput;
  state.hram.joypad.hJoyLast = realInput;
}

export function getJoypad(state: GameState): void {
  const lastFrameMirror = state.hram.joypad.hJoyDown;
  const realInput = state.hram.joypad.hJoypadDown;
  const delta = lastFrameMirror ^ realInput;

  state.hram.joypad.hJoyReleased = delta & lastFrameMirror;
  state.hram.joypad.hJoyPressed = delta & realInput;
  state.hram.joypad.hJoyDown = realInput;
}

export function joyTextDelay(state: GameState): void {
  getJoypad(state);
  state.hram.joypad.hJoyLast =
    state.hram.hInMenu
      ? state.hram.joypad.hJoyDown
      : state.hram.joypad.hJoyPressed;

  if (state.hram.joypad.hJoyPressed !== 0) {
    state.wram.wTextDelayFrames = 15;
  } else if (state.wram.wTextDelayFrames !== 0) {
    state.hram.joypad.hJoyLast = 0;
  } else {
    state.wram.wTextDelayFrames = 5;
  }
}

export function copyBytes(dest: Uint8Array, source: Uint8Array, length: number): void {
  for (let i = 0; i < length; i++) {
    dest[i] = source[i];
  }
}

export function swapBytes(arr1: Uint8Array, arr2: Uint8Array, length: number): void {
  for (let i = 0; i < length; i++) {
    [arr1[i], arr2[i]] = [arr2[i], arr1[i]];
  }
}

export function byteFill(dest: Uint8Array, length: number, value: number): void {
  for (let i = 0; i < length; i++) {
    dest[i] = value;
  }
}

export function simpleMultiply(a: number, c: number): number {
  let result = 0;
  for (let i = 0; i < a; i++) {
    result += c;
  }
  return result;
}

export function simpleDivide(a: number, c: number): [number, number] {
  let quotient = 0;
  while (a >= c) {
    a -= c;
    quotient += 1;
  }
  return [quotient, a];
}

export function isStringBlank(s: string): boolean {
  for (const char of s) {
    if (char !== ' ') {
      return false;
    }
  }
  return true;
}

export function initString(currentString: string, newString: string, length: number): string {
  if (isStringBlank(currentString)) {
    return newString.padEnd(length, ' ').slice(0, length);
  }
  return currentString;
}

export function multiply(multiplicand: Uint8Array, multiplier: number): Uint8Array {
  const mVal = new DataView(multiplicand.buffer).getUint32(0, false);
  const pVal = mVal * multiplier;
  const result = new Uint8Array(4);
  new DataView(result.buffer).setUint32(0, pVal, false);
  return result;
}

export function divide(dividend: Uint8Array, divisor: number): Uint8Array {
  const dVal = new DataView(dividend.buffer).getUint32(0, false);
  const qVal = Math.floor(dVal / divisor);
  const result = new Uint8Array(4);
  new DataView(result.buffer).setUint32(0, qVal, false);
  return result;
}

export function subtractAbsolute(a: number, b: number): [number, boolean] {
  if (a >= b) {
    return [a - b, false];
  } else {
    return [b - a, true];
  }
}

export function getTmhmMove(tmhmId: number): MoveName {
  return TMHM_MOVES[tmhmId];
}

export function getTypeName(typeEnum: PokemonType): string {
  return typeEnum;
}

export function getMoveName(moveEnum: MoveName): string {
  return moveEnum;
}

const LANDMARK_ID_BY_CONSTANT = Object.fromEntries(
  POKEGEAR_LANDMARKS.map(entry => [entry.constant, entry.id])
) as Record<string, number>;
const LANDMARK_ENTRY_BY_ID: Record<number, LandmarkEntry> = Object.fromEntries(
  POKEGEAR_LANDMARKS.map(entry => [entry.id, entry])
) as Record<number, LandmarkEntry>;
const MAP_LANDMARK_LOOKUP = Object.fromEntries(
  Object.entries(MAP_TO_LANDMARK).map(([name, constant]) => [
    name.toUpperCase(),
    LANDMARK_ID_BY_CONSTANT[constant] ?? LANDMARK_SPECIAL,
  ])
) as Record<string, number>;


export function getPokemonName(pokemonId: number): string {
  return Pokemon[pokemonId];
}

// ASM mapping: pokecrystal_disassembly/home/map.asm (GetWorldMapLocation).
export function getWorldMapLocation(mapGroup: number, mapId: number): number {
  const metadata = getMapMetadataByGroup(mapGroup, mapId);
  if (!metadata) {
    return LANDMARK_SPECIAL;
  }

  const landmark = MAP_LANDMARK_LOOKUP[metadata.name.toUpperCase()];
  if (landmark !== undefined) {
    return landmark;
  }

  const fallback = MAP_LANDMARK_LOOKUP[metadata.constant.toUpperCase()];
  if (fallback !== undefined) {
    return fallback;
  }

  return LANDMARK_SPECIAL;
}

// ASM mapping: pokecrystal_disassembly/home/region.asm (IsInJohto).
export function isInJohto(state: GameState): number {
  let mapGroup = state.wram.wMapGroup;
  let mapId = state.wram.wMapNumber;
  let location = getWorldMapLocation(mapGroup, mapId);

  if (location === LANDMARK_FAST_SHIP) {
    return Region.JOHTO;
  }

  if (location === LANDMARK_SPECIAL) {
    mapGroup = state.wram.wBackupMapGroup;
    mapId = state.wram.wBackupMapNumber;
    location = getWorldMapLocation(mapGroup, mapId);
  }

  const entry = LANDMARK_ENTRY_BY_ID[location];
  if (entry?.region === 'KANTO') {
    return Region.KANTO;
  }

  return Region.JOHTO;
}
