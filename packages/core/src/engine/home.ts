
import { GameState } from '../core/state';
import { HardwareRNG } from './games/rng';
import { MoveName } from '../core/enums/move';
import { PokemonType } from '../core/enums/pokemon';
import { getPokegearLandmarks, getMapToLandmark, LandmarkEntry } from '@pokecrystal/assets/content';
import { LANDMARK_FAST_SHIP, LANDMARK_SPECIAL, Pokemon, Region, TMHM_MOVES } from '../core/constants';
import { GB_FRAME_DURATION_SECONDS } from '../core/gb-timing';
import { getMapMetadataByGroup } from './world/maps';


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
  const realInput = rng.nextByte();

  const lastFrame = state.hram.joypad.hJoyDown;
  const delta = lastFrame ^ realInput;

  state.hram.joypad.hJoyReleased = delta & lastFrame;
  state.hram.joypad.hJoyPressed = delta & realInput;
  state.hram.joypad.hJoyDown = realInput;
  state.hram.joypad.hJoyLast = realInput;
}

export function getJoypad(state: GameState): void {
  const lastFrameMirror = state.hram.joypad.hJoyDown;
  const realInput = state.hram.joypad.hJoyDown;

  const delta = lastFrameMirror ^ realInput;
  state.hram.joypad.hJoyReleased = delta & lastFrameMirror;
  state.hram.joypad.hJoyPressed = delta & realInput;
  state.hram.joypad.hJoyDown = realInput;
}

export function joyTextDelay(state: GameState): void {
  getJoypad(state);
  state.hram.joypad.hJoyLast =
    state.hram.hInMenu
      ? state.hram.joypad.hJoyPressed
      : state.hram.joypad.hJoyDown;

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
  return /^\s*$/.test(s);
}

export function initString(currentString: string, newString: string, length: number): string {
  if (isStringBlank(currentString)) {
    return newString.padEnd(length, ' ').substring(0, length);
  }
  return currentString;
}

function toBigIntBE(bytes: Uint8Array): bigint {
    let result = 0n;
    for (const byte of bytes) {
        result = (result << 8n) + BigInt(byte);
    }
    return result;
}

function fromBigIntBE(value: bigint, byteLength: number): Uint8Array {
    const result = new Uint8Array(byteLength);
    for (let i = byteLength - 1; i >= 0; i--) {
        result[i] = Number(value & 0xffn);
        value >>= 8n;
    }
    return result;
}

export function multiply(multiplicand: Uint8Array, multiplier: number): Uint8Array {
  const mVal = toBigIntBE(multiplicand);
  const pVal = mVal * BigInt(multiplier);
  return fromBigIntBE(pVal, 4);
}

export function divide(dividend: Uint8Array, divisor: number): Uint8Array {
    const dVal = toBigIntBE(dividend);
    const qVal = dVal / BigInt(divisor);
    return fromBigIntBE(qVal, 4);
}

export function subtractAbsolute(a: number, b: number): [number, boolean] {
  if (a >= b) {
    return [a - b, false];
  } else {
    return [b - a, true];
  }
}

export function getTmhmMove(tmhmId: number): MoveName {
  return TMHM_MOVES[tmhmId] as MoveName;
}

export function getTypeName(typeEnum: PokemonType): string {
  return typeEnum;
}

export function getMoveName(moveEnum: MoveName): string {
  return moveEnum;
}

export function getPokemonName(pokemonId: number): string {
  return Pokemon[pokemonId] ?? `Pokemon #${pokemonId}`;
}

let landmarkIdByConstant: Record<string, number> | null = null;
let landmarkEntryById: Record<number, LandmarkEntry> | null = null;
let mapLandmarkLookup: Record<string, number> | null = null;

async function loadLandmarkData() {
    if (landmarkIdByConstant && landmarkEntryById && mapLandmarkLookup) {
        return;
    }
    const landmarks = await getPokegearLandmarks();
    const mapToLandmark = await getMapToLandmark();

    const landmarkIdByConstant_local = Object.fromEntries(landmarks.map((entry: LandmarkEntry) => [entry.constant, entry.id]));
    landmarkIdByConstant = landmarkIdByConstant_local
    landmarkEntryById = Object.fromEntries(landmarks.map((entry: LandmarkEntry) => [entry.id, entry]));
    mapLandmarkLookup = Object.fromEntries(Object.entries(mapToLandmark).map(
        ([name, constant]: [string, string]) => [name.toUpperCase(), landmarkIdByConstant_local[constant] ?? LANDMARK_SPECIAL]
    ));
}

export async function getWorldMapLocation(mapGroup: number, mapId: number): Promise<number> {
    await loadLandmarkData();
    const metadata = getMapMetadataByGroup(mapGroup, mapId);
    if (!metadata) {
        return LANDMARK_SPECIAL;
    }

    if (!mapLandmarkLookup) {
        return LANDMARK_SPECIAL;
    }

    let landmark = mapLandmarkLookup[metadata.name.toUpperCase()];
    if (landmark !== undefined) {
        return landmark;
    }

    landmark = mapLandmarkLookup[metadata.constant.toUpperCase()];
    if (landmark !== undefined) {
        return landmark;
    }

    return LANDMARK_SPECIAL;
}

export async function isInJohto(state: GameState): Promise<number> {
    let location = await getWorldMapLocation(state.wram.wMapGroup, state.wram.wMapNumber);

    if (location === LANDMARK_FAST_SHIP) {
        return Region.JOHTO;
    }

    if (location === LANDMARK_SPECIAL) {
        location = await getWorldMapLocation(state.wram.wBackupMapGroup, state.wram.wBackupMapNumber);
    }

    await loadLandmarkData();
    if (!landmarkEntryById) {
        return Region.JOHTO;
    }
    const entry = landmarkEntryById[location];
    if (entry && entry.region === "KANTO") {
        return Region.KANTO;
    }

    return Region.JOHTO;
}
