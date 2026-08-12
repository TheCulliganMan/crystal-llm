import fs from "fs";
import path from "path";
import {
  pcmClipToBytes,
  renderPcmClipFromAsm,
} from "@pokecrystal/core/audio-export/pcm-clip";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { ensureDir, getTypeScriptDataDir } from "./asm-utils";

type PcmAudioFormat = {
  sample_rate_hz: number;
  channels: number;
  bits_per_sample: number;
};

type ExportedAudioAssetBase = {
  id: string;
  path: string;
  kind: "music" | "sound_effect" | "cry";
};

export type ExportedAudioAsset = ExportedAudioAssetBase & (
  | {
      source: "midi";
    }
  | {
      source: "pcm";
      pcm_format: PcmAudioFormat;
      pcm_frame_count: number;
      payload_hash: string;
      loop_start_sample: number | null;
      loop_end_sample: number | null;
    }
);

type RenderedPcmAudioAsset = ExportedAudioAssetBase & {
  source: "pcm";
  pcm_format: PcmAudioFormat;
  pcm_frame_count: number;
  payload_hash: string;
  loop_start_sample: number | null;
  loop_end_sample: number | null;
};

const CORE_AUDIO_ROOT = "content-packs/core-modular";
const PCM_AUDIO_FORMAT: PcmAudioFormat = {
  sample_rate_hz: 44_100,
  channels: 2,
  bits_per_sample: 16,
};

export type ExportedPokemonCryMetadata = {
  cry: string;
  pitch: number;
  length: number;
};

export function pokemonCryVariantIds(speciesId: string): [string, string, string] {
  if (!/^[A-Z0-9_]+$/.test(speciesId)) {
    throw new Error(`Pokemon cry species id must be an exact constant token, got '${speciesId}'.`);
  }
  return [
    `CRY_MON_${speciesId}`,
    `CRY_MON_${speciesId}_GROWL`,
    `CRY_MON_${speciesId}_ROAR`,
  ];
}

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

type PcmPayloadMetadata = Pick<
  RenderedPcmAudioAsset,
  "pcm_frame_count" | "payload_hash" | "loop_start_sample" | "loop_end_sample"
>;

function fnv1a32(bytes: Uint8Array): string {
  let hash = 0x811c9dc5;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

function writeRenderedPcmAsset(
  asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata>,
  disassemblyRoot: string,
  stem: string,
  cryParameters?: { cryPitch: number; cryLength: number },
): PcmPayloadMetadata {
  const kind = asset.kind === "music" ? "music" : asset.kind === "cry" ? "cry" : "sfx";
  let clip;
  try {
    clip = renderPcmClipFromAsm(
      path.join(disassemblyRoot, "audio"),
      kind,
      stem,
      asset.id,
      cryParameters,
    );
  } catch (error) {
    throw new Error(`Audio asset ${asset.id} failed to render '${stem}': ${String(error)}`);
  }
  if (!clip) {
    throw new Error(`Audio asset ${asset.id} has no exact ASM program for '${stem}'.`);
  }
  if (
    clip.sampleRate !== asset.pcm_format.sample_rate_hz ||
    clip.pcm.length === 0 ||
    clip.pcm.length % asset.pcm_format.channels !== 0
  ) {
    throw new Error(`Audio asset ${asset.id} rendered an invalid PCM payload.`);
  }
  const absolutePath = path.join(getTypeScriptDataDir(), asset.path);
  ensureDir(path.dirname(absolutePath));
  const bytes = pcmClipToBytes(clip);
  fs.writeFileSync(absolutePath, Buffer.from(bytes));
  return {
    pcm_frame_count: clip.pcm.length / asset.pcm_format.channels,
    payload_hash: fnv1a32(bytes),
    loop_start_sample: clip.loopStartSample,
    loop_end_sample: clip.loopEndSample,
  };
}

function writeSilentPcmAsset(
  asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata>,
): PcmPayloadMetadata {
  const absolutePath = path.join(getTypeScriptDataDir(), asset.path);
  ensureDir(path.dirname(absolutePath));
  const bytes = Buffer.alloc(asset.pcm_format.channels * 2);
  fs.writeFileSync(absolutePath, bytes);
  return {
    pcm_frame_count: 1,
    payload_hash: fnv1a32(bytes),
    loop_start_sample: null,
    loop_end_sample: null,
  };
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
      const asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata> = {
        id,
        path: `${CORE_AUDIO_ROOT}/music/${id}.pcm`,
        kind: "music",
        source: "pcm",
        pcm_format: PCM_AUDIO_FORMAT,
      };
      const metadata = id === "MUSIC_NONE"
        ? writeSilentPcmAsset(asset)
        : writeRenderedPcmAsset(asset, disassemblyRoot, stem);
      return { ...asset, ...metadata } satisfies RenderedPcmAudioAsset;
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
    const asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata> = {
      id,
      path: `${CORE_AUDIO_ROOT}/sfx/${id}.pcm`,
      kind: "sound_effect",
      source: "pcm",
      pcm_format: PCM_AUDIO_FORMAT,
    };
    const metadata = writeRenderedPcmAsset(asset, disassemblyRoot, label.replace(/^Sfx_/, ""));
    return { ...asset, ...metadata } satisfies RenderedPcmAudioAsset;
  });

  const cries = uniqueSorted([
    ...Object.values(pokemonCries).map((cry) => requireExactCryLabel(cry.cry)),
    ...exportCryLabelsFromAsm(),
  ]).map((label) => {
    const asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata> = {
      id: label,
      path: `${CORE_AUDIO_ROOT}/cries/${label}.pcm`,
      kind: "cry",
      source: "pcm",
      pcm_format: PCM_AUDIO_FORMAT,
    };
    const metadata = writeRenderedPcmAsset(asset, disassemblyRoot, label.replace(/^CRY_/, ""));
    return { ...asset, ...metadata } satisfies RenderedPcmAudioAsset;
  });

  const speciesCries = Object.entries(pokemonCries)
    .sort(([left], [right]) => left.localeCompare(right))
    .flatMap(([speciesId, cry]) => {
      const stem = requireExactCryLabel(cry.cry).replace(/^CRY_/, "");
      return pokemonCryVariantIds(speciesId).map((id, index) => {
        const lengthOffset = [0, 0x00c0, 0x0040][index];
        const asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata> = {
          id,
          path: `${CORE_AUDIO_ROOT}/cries/${id}.pcm`,
          kind: "cry",
          source: "pcm",
          pcm_format: PCM_AUDIO_FORMAT,
        };
        const metadata = writeRenderedPcmAsset(asset, disassemblyRoot, stem, {
          cryPitch: cry.pitch,
          cryLength: cry.length + lengthOffset,
        });
        return { ...asset, ...metadata } satisfies RenderedPcmAudioAsset;
      });
    });

  const assets: Record<string, ExportedAudioAsset> = {};
  for (const asset of [...music, ...sfx, ...cries, ...speciesCries]) {
    if (Object.prototype.hasOwnProperty.call(assets, asset.id)) {
      throw new Error(`duplicate audio asset id ${asset.id}`);
    }
    assets[asset.id] = asset;
  }
  return assets;
}
