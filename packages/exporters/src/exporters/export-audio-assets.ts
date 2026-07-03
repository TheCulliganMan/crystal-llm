import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { ensureDir, getTypeScriptDataDir } from "./asm-utils";

export type ExportedAudioAsset = {
  id: string;
  path: string;
  kind: "music" | "sound_effect" | "cry";
  source: "midi";
};

const CORE_AUDIO_ROOT = "content-packs/core-modular";
const MIDI_TICKS_PER_QUARTER = 96;

export type ExportedPokemonCryMetadata = {
  cry: string;
  pitch: number;
  length: number;
};

const uniqueSorted = (values: Iterable<string>): string[] => Array.from(new Set(values)).sort();

function exportConstantsFromAsm(disassemblyRoot: string, relativePath: string, prefix: "MUSIC" | "SFX"): string[] {
  const source = fs.readFileSync(path.join(disassemblyRoot, relativePath), "utf8");
  return Array.from(source.matchAll(new RegExp(`^\\s*const\\s+(${prefix}_[A-Z0-9_]+)\\b`, "gm"))).map(
    (match) => match[1]
  );
}

function exportPointerLabelsFromAsm(disassemblyRoot: string, relativePath: string): string[] {
  const source = fs.readFileSync(path.join(disassemblyRoot, relativePath), "utf8");
  return Array.from(source.matchAll(/^\s*dba\s+([A-Za-z_][A-Za-z0-9_]*)\b/gm)).map((match) => match[1]);
}

function exportMusicLabelStemsFromAsm(disassemblyRoot: string): Map<string, string> {
  const musicDir = path.join(disassemblyRoot, "audio", "music");
  const labels = new Map<string, string>();
  for (const entry of fs
    .readdirSync(musicDir)
    .filter((fileName) => fileName.endsWith(".asm"))
    .sort()) {
    const stem = entry.replace(/\.asm$/, "");
    const source = fs.readFileSync(path.join(musicDir, entry), "utf8");
    for (const match of source.matchAll(/^([A-Za-z_][A-Za-z0-9_]*):/gm)) {
      if (labels.has(match[1])) {
        throw new Error(`Music label '${match[1]}' is declared by multiple ASM files.`);
      }
      labels.set(match[1], stem);
    }
  }
  return labels;
}

function writeU16(value: number): Buffer {
  const buffer = Buffer.alloc(2);
  buffer.writeUInt16BE(value);
  return buffer;
}

function writeU32(value: number): Buffer {
  const buffer = Buffer.alloc(4);
  buffer.writeUInt32BE(value);
  return buffer;
}

function midiVarLen(value: number): Buffer {
  if (!Number.isInteger(value) || value < 0 || value > 0x0fffffff) {
    throw new Error(`MIDI delta time ${value} is outside variable-length range.`);
  }
  const bytes = [value & 0x7f];
  value >>= 7;
  while (value > 0) {
    bytes.unshift((value & 0x7f) | 0x80);
    value >>= 7;
  }
  return Buffer.from(bytes);
}

function exactAudioSeed(id: string): number {
  let seed = 0;
  for (const byte of Buffer.from(id, "ascii")) {
    seed = (seed * 131 + byte) >>> 0;
  }
  return seed;
}

function midiNoteForAudioId(id: string, offset = 0): number {
  return 48 + ((exactAudioSeed(id) + offset * 7) % 36);
}

function standardMidiPayload(id: string, kind: ExportedAudioAsset["kind"]): Buffer {
  const channel = kind === "music" ? 0 : kind === "sound_effect" ? 1 : 2;
  const velocity = kind === "music" ? 84 : 104;
  const duration = kind === "music" ? MIDI_TICKS_PER_QUARTER : Math.floor(MIDI_TICKS_PER_QUARTER / 2);
  const noteCount = kind === "music" ? 4 : 1;
  const trackEvents: Buffer[] = [
    Buffer.from([0x00, 0xff, 0x51, 0x03, 0x07, 0xa1, 0x20]),
    Buffer.from([0x00, 0xc0 | channel, kind === "cry" ? 80 : kind === "sound_effect" ? 81 : 1]),
  ];
  for (let index = 0; index < noteCount; index += 1) {
    const note = midiNoteForAudioId(id, index);
    trackEvents.push(Buffer.from([0x00, 0x90 | channel, note, velocity]));
    trackEvents.push(Buffer.concat([midiVarLen(duration), Buffer.from([0x80 | channel, note, 0x00])]));
  }
  trackEvents.push(Buffer.from([0x00, 0xff, 0x2f, 0x00]));
  const track = Buffer.concat(trackEvents);
  return Buffer.concat([
    Buffer.from("MThd", "ascii"),
    writeU32(6),
    writeU16(0),
    writeU16(1),
    writeU16(MIDI_TICKS_PER_QUARTER),
    Buffer.from("MTrk", "ascii"),
    writeU32(track.length),
    track,
  ]);
}

function writeGeneratedMidiAsset(asset: ExportedAudioAsset): void {
  const absolutePath = path.join(getTypeScriptDataDir(), asset.path);
  ensureDir(path.dirname(absolutePath));
  fs.writeFileSync(absolutePath, standardMidiPayload(asset.id, asset.kind));
}

function exportIndexedAudioPointers(
  disassemblyRoot: string,
  constantsPath: string,
  pointersPath: string,
  prefix: "MUSIC" | "SFX"
): Array<{ id: string; label: string }> {
  const constants = exportConstantsFromAsm(disassemblyRoot, constantsPath, prefix);
  const labels = exportPointerLabelsFromAsm(disassemblyRoot, pointersPath);
  if (constants.length !== labels.length) {
    throw new Error(
      `${prefix} constants and pointer table lengths differ: ${constants.length} constants, ${labels.length} pointers.`
    );
  }
  return constants.map((id, index) => ({ id, label: labels[index] }));
}

function requireExactCryLabel(label: unknown): string {
  if (typeof label !== "string" || !/^CRY_[A-Z0-9_]+$/.test(label)) {
    throw new Error(`Pokemon cry metadata must use exact CRY_* labels, got '${String(label)}'.`);
  }
  return label;
}

export function exportCryLabelsFromAsm(disassemblyRoot = getDisassemblyRoot()): string[] {
  const constantsPath = path.join(disassemblyRoot, "constants", "cry_constants.asm");
  const source = fs.readFileSync(constantsPath, "utf8");
  return uniqueSorted(
    Array.from(source.matchAll(/^\s*const\s+(CRY_[A-Z0-9_]+)\b/gm)).map((match) => requireExactCryLabel(match[1]))
  );
}

export function exportPokemonCryMetadataFromAsm(
  speciesIds: Iterable<string>,
  disassemblyRoot = getDisassemblyRoot()
): Record<string, ExportedPokemonCryMetadata> {
  const speciesSet = new Set(speciesIds);
  const sourcePath = path.join(disassemblyRoot, "data", "pokemon", "cries.asm");
  const source = fs.readFileSync(sourcePath, "utf8");
  const pokemonCries: Record<string, ExportedPokemonCryMetadata> = {};

  for (const line of source.split(/\r?\n/)) {
    const match = line.match(/^\s*mon_cry\s+([^,]+),\s*([^,]+),\s*([^;]+);\s*([A-Z0-9_]+)\s*$/);
    if (!match) {
      continue;
    }
    const speciesId = match[4];
    if (!speciesSet.has(speciesId)) {
      continue;
    }
    pokemonCries[speciesId] = {
      cry: requireExactCryLabel(match[1].trim()),
      pitch: parseExactInteger(match[2].trim(), speciesId),
      length: parseExactInteger(match[3].trim(), speciesId),
    };
  }

  for (const speciesId of speciesSet) {
    if (!pokemonCries[speciesId]) {
      throw new Error(`Pokemon cry metadata is missing exact species '${speciesId}'.`);
    }
  }

  return pokemonCries;
}

function parseExactInteger(value: string, speciesId: string): number {
  if (!/^-?\d+$/.test(value)) {
    throw new Error(`Pokemon cry metadata for '${speciesId}' has non-integer value '${value}'.`);
  }
  return Number(value);
}

export function exportAudioAssets(
  pokemonCries: Record<string, ExportedPokemonCryMetadata>,
  disassemblyRoot = getDisassemblyRoot()
): Record<string, ExportedAudioAsset> {
  const musicLabelStems = exportMusicLabelStemsFromAsm(disassemblyRoot);
  const music = exportIndexedAudioPointers(
    disassemblyRoot,
    "constants/music_constants.asm",
    "audio/music_pointers.asm",
    "MUSIC"
  )
    .map(({ id, label }) => {
      const stem = musicLabelStems.get(label);
      if (!stem) {
        throw new Error(`Music pointer '${label}' for '${id}' does not match an exact audio/music ASM label.`);
      }
      return {
        id,
        path: `${CORE_AUDIO_ROOT}/music/${id}.mid`,
        kind: "music" as const,
        source: "midi" as const,
      };
    });

  const sfx = exportIndexedAudioPointers(
    disassemblyRoot,
    "constants/sfx_constants.asm",
    "audio/sfx_pointers.asm",
    "SFX"
  ).map(({ id, label }) => {
    if (!label.startsWith("Sfx_")) {
      throw new Error(`SFX pointer '${label}' for '${id}' must use an exact Sfx_* label.`);
    }
    return {
      id,
      path: `${CORE_AUDIO_ROOT}/sfx/${id}.mid`,
      kind: "sound_effect" as const,
      source: "midi" as const,
    };
  });

  const cries = uniqueSorted([
    ...Object.values(pokemonCries).map((cry) => requireExactCryLabel(cry.cry)),
    ...exportCryLabelsFromAsm(),
  ]).map((label) => ({
    id: label,
    path: `${CORE_AUDIO_ROOT}/cries/${label}.mid`,
    kind: "cry" as const,
    source: "midi" as const,
  }));

  const assets: Record<string, ExportedAudioAsset> = {};
  for (const asset of [...music, ...sfx, ...cries]) {
    if (Object.prototype.hasOwnProperty.call(assets, asset.id)) {
      throw new Error(`duplicate audio asset id ${asset.id}`);
    }
    writeGeneratedMidiAsset(asset);
    assets[asset.id] = asset;
  }
  return assets;
}
