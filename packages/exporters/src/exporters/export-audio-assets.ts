import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";

export type ExportedAudioAsset = {
  id: string;
  path: string;
  kind: "music" | "sound_effect" | "cry";
  source: "midi";
};

const CORE_AUDIO_ROOT = "content-packs/core-modular";

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
    assets[asset.id] = asset;
  }
  return assets;
}
