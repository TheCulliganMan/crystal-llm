import fs from "fs";
import path from "path";
import { createHash } from "crypto";
import {
  loadPcmRenderContext,
  pcmClipToBytes,
  renderPcmClipFromAsm,
  type PcmRenderContext,
} from "@pokecrystal/core/audio-export/pcm-clip";
import { PcmConverter } from "@pokecrystal/core/audio-export/converter";
import {
  buildCrystalMidi,
  countMidiNoteOnEvents,
  parseCrystalMidi,
} from "@pokecrystal/core/audio-export/crystal-midi";
import { buildAsmAudioProgram } from "@pokecrystal/core/audio-export/asm-programs";
import {
  AsmAudioParser,
  type ParsedMusicData,
} from "@pokecrystal/core/audio-export/parsers";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import {
  ensureDir,
  getTypeScriptDataDir,
  writeJsonToTargets,
} from "./asm-utils";

type PcmAudioFormat = {
  sample_rate_hz: number;
  channels: number;
  bits_per_sample: number;
};

type ExportedAudioAssetBase = {
  id: string;
  path: string;
  kind: "music" | "sound_effect" | "cry";
  sfx_priority?: number;
};

export type ExportedAudioAsset = ExportedAudioAssetBase & {
  source: "pcm";
  pcm_format: PcmAudioFormat;
  pcm_frame_count: number;
  payload_hash: string;
  loop_start_sample: number | null;
  loop_end_sample: number | null;
  midi_program?: ExportedMidiAudioProgram;
};

export type ExportedMidiAudioProgram = {
  profile: "pokecrystal-midi-v1";
  midi_base64: string;
};

type RenderedPcmAudioAsset = ExportedAudioAssetBase & {
  source: "pcm";
  pcm_format: PcmAudioFormat;
  pcm_frame_count: number;
  payload_hash: string;
  loop_start_sample: number | null;
  loop_end_sample: number | null;
  midi_program: ExportedMidiAudioProgram;
};

const CORE_AUDIO_ROOT = "content-packs/core-modular";
const PCM_AUDIO_FORMAT: PcmAudioFormat = {
  sample_rate_hz: 22_050,
  channels: 2,
  bits_per_sample: 16,
};

const AUDIO_CACHE_SCHEMA = 1;
const AUDIO_RENDERER_FORMAT = "asm-accurate-pcm-s16le-22050-stereo-v1";

type AudioCacheManifest = {
  schema: number;
  renderer_format: string;
  source_sha256: string;
};

function audioCachePath(): string {
  return path.resolve(
    getTypeScriptDataDir(),
    "..",
    "..",
    ".cache",
    "core-modular-audio-export.json",
  );
}

function audioSourceFingerprint(disassemblyRoot: string): string {
  const files = [
    "constants/cry_constants.asm",
    "constants/music_constants.asm",
    "constants/sfx_constants.asm",
    "data/pokemon/cries.asm",
  ];
  const visit = (directory: string): void => {
    for (const entry of fs
      .readdirSync(path.join(disassemblyRoot, directory), {
        withFileTypes: true,
      })
      .sort((left, right) => left.name.localeCompare(right.name))) {
      const relativePath = path.posix.join(directory, entry.name);
      if (entry.isDirectory()) {
        visit(relativePath);
      } else if (entry.isFile() && entry.name.endsWith(".asm")) {
        files.push(relativePath);
      }
    }
  };
  visit("audio");
  const hash = createHash("sha256");
  hash.update(`${AUDIO_RENDERER_FORMAT}\0`);
  for (const relativePath of files.sort()) {
    hash.update(`${relativePath}\0`);
    hash.update(fs.readFileSync(path.join(disassemblyRoot, relativePath)));
    hash.update("\0");
  }
  return hash.digest("hex");
}

function expectedAudioCacheManifest(
  disassemblyRoot: string,
): AudioCacheManifest {
  return {
    schema: AUDIO_CACHE_SCHEMA,
    renderer_format: AUDIO_RENDERER_FORMAT,
    source_sha256: audioSourceFingerprint(disassemblyRoot),
  };
}

function cacheManifestMatches(expected: AudioCacheManifest): boolean {
  try {
    const actual = JSON.parse(
      fs.readFileSync(audioCachePath(), "utf8"),
    ) as AudioCacheManifest;
    return (
      actual.schema === expected.schema &&
      actual.renderer_format === expected.renderer_format &&
      actual.source_sha256 === expected.source_sha256
    );
  } catch {
    return false;
  }
}

export type ExportedPokemonCryMetadata = {
  cry: string;
  pitch: number;
  length: number;
};

export function pokemonCryVariantIds(
  speciesId: string,
): [string, string, string] {
  if (!/^[A-Z0-9_]+$/.test(speciesId)) {
    throw new Error(
      `Pokemon cry species id must be an exact constant token, got '${speciesId}'.`,
    );
  }
  return [
    `CRY_MON_${speciesId}`,
    `CRY_MON_${speciesId}_GROWL`,
    `CRY_MON_${speciesId}_ROAR`,
  ];
}

const uniqueSorted = (values: Iterable<string>): string[] =>
  Array.from(new Set(values)).sort();

function exportConstantsFromAsm(
  disassemblyRoot: string,
  relativePath: string,
  prefix: "MUSIC" | "SFX",
): string[] {
  const source = fs.readFileSync(
    path.join(disassemblyRoot, relativePath),
    "utf8",
  );
  return Array.from(
    source.matchAll(
      new RegExp(`^\\s*const\\s+(${prefix}_[A-Z0-9_]+)\\b`, "gm"),
    ),
  ).map((match) => match[1]);
}

function exportPointerLabelsFromAsm(
  disassemblyRoot: string,
  relativePath: string,
): string[] {
  const source = fs.readFileSync(
    path.join(disassemblyRoot, relativePath),
    "utf8",
  );
  return Array.from(
    source.matchAll(/^\s*dba\s+([A-Za-z_][A-Za-z0-9_]*)\b/gm),
  ).map((match) => match[1]);
}

function exportMusicLabelStemsFromAsm(
  disassemblyRoot: string,
): Map<string, string> {
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
        throw new Error(
          `Music label '${match[1]}' is declared by multiple ASM files.`,
        );
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

export function downsampleStereoPcm(
  input: Int16Array,
  loopStartSample: number | null,
  loopEndSample: number | null,
): {
  pcm: Int16Array;
  loopStartSample: number | null;
  loopEndSample: number | null;
} {
  if (input.length % 2 !== 0) {
    throw new Error("44.1 kHz stereo PCM must contain complete frames");
  }
  const inputFrames = input.length / 2;
  const outputFrames = Math.ceil(inputFrames / 2);
  const pcm = new Int16Array(outputFrames * 2);
  for (
    let inputFrame = 0, outputFrame = 0;
    inputFrame < inputFrames;
    inputFrame += 2, outputFrame += 1
  ) {
    pcm[outputFrame * 2] = input[inputFrame * 2];
    pcm[outputFrame * 2 + 1] = input[inputFrame * 2 + 1];
  }
  return {
    pcm,
    loopStartSample:
      loopStartSample == null ? null : Math.ceil(loopStartSample / 2),
    loopEndSample:
      loopEndSample == null ? null : Math.ceil(loopEndSample / 2),
  };
}

function fnv1a32(bytes: Uint8Array): string {
  let hash = 0x811c9dc5;
  for (const byte of bytes) {
    hash ^= byte;
    hash = Math.imul(hash, 0x01000193) >>> 0;
  }
  return hash.toString(16).padStart(8, "0");
}

function readCachedPcmMetadata(
  asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata | "midi_program">,
): PcmPayloadMetadata | null {
  try {
    const absolutePath = path.join(getTypeScriptDataDir(), asset.path);
    const metadataPath = absolutePath.replace(/\.pcm$/, ".json");
    const payload = JSON.parse(fs.readFileSync(metadataPath, "utf8")) as Record<
      string,
      RenderedPcmAudioAsset
    >;
    const cached = payload[asset.id];
    if (
      !cached ||
      cached.id !== asset.id ||
      cached.path !== asset.path ||
      cached.kind !== asset.kind ||
      cached.source !== "pcm" ||
      cached.pcm_format.sample_rate_hz !== PCM_AUDIO_FORMAT.sample_rate_hz ||
      cached.pcm_format.channels !== PCM_AUDIO_FORMAT.channels ||
      cached.pcm_format.bits_per_sample !== PCM_AUDIO_FORMAT.bits_per_sample ||
      !Number.isInteger(cached.pcm_frame_count) ||
      cached.pcm_frame_count <= 0 ||
      !/^[0-9a-f]{8}$/.test(cached.payload_hash)
    ) {
      return null;
    }
    const bytes = fs.readFileSync(absolutePath);
    if (
      bytes.length !== cached.pcm_frame_count * PCM_AUDIO_FORMAT.channels * 2 ||
      fnv1a32(bytes) !== cached.payload_hash ||
      (cached.loop_start_sample == null) !== (cached.loop_end_sample == null) ||
      (cached.loop_start_sample != null &&
        (cached.loop_start_sample < 0 ||
          cached.loop_end_sample! <= cached.loop_start_sample ||
          cached.loop_end_sample! > cached.pcm_frame_count))
    ) {
      return null;
    }
    return {
      pcm_frame_count: cached.pcm_frame_count,
      payload_hash: cached.payload_hash,
      loop_start_sample: cached.loop_start_sample ?? null,
      loop_end_sample: cached.loop_end_sample ?? null,
    };
  } catch {
    return null;
  }
}

function writeRenderedPcmAsset(
  asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata | "midi_program">,
  disassemblyRoot: string,
  stem: string,
  cryParameters?: { cryPitch: number; cryLength: number },
  reuseCache = false,
): PcmPayloadMetadata {
  if (reuseCache) {
    const cached = readCachedPcmMetadata(asset);
    if (cached) {
      return cached;
    }
  }
  const kind =
    asset.kind === "music" ? "music" : asset.kind === "cry" ? "cry" : "sfx";
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
    throw new Error(
      `Audio asset ${asset.id} failed to render '${stem}': ${String(error)}`,
    );
  }
  if (!clip) {
    throw new Error(
      `Audio asset ${asset.id} has no exact ASM program for '${stem}'.`,
    );
  }
  if (
    clip.sampleRate !== 44_100 ||
    clip.pcm.length === 0 ||
    clip.pcm.length % 2 !== 0
  ) {
    throw new Error(`Audio asset ${asset.id} rendered an invalid PCM payload.`);
  }
  const downsampled = downsampleStereoPcm(
    clip.pcm,
    clip.loopStartSample,
    clip.loopEndSample,
  );
  const { pcm } = downsampled;
  const outputFrames = pcm.length / 2;
  const absolutePath = path.join(getTypeScriptDataDir(), asset.path);
  ensureDir(path.dirname(absolutePath));
  const bytes = pcmClipToBytes({ pcm });
  fs.writeFileSync(absolutePath, Buffer.from(bytes));
  const metadata = {
    pcm_frame_count: outputFrames,
    payload_hash: fnv1a32(bytes),
    loop_start_sample: downsampled.loopStartSample,
    loop_end_sample: downsampled.loopEndSample,
  };
  writeJsonToTargets(
    asset.path.replace(/\.pcm$/, ".json"),
    { [asset.id]: { ...asset, ...metadata } },
    { indent: 2 },
  );
  return metadata;
}

function buildMidiAudioProgram(
  disassemblyRoot: string,
  kind: "music" | "sfx" | "cry",
  stem: string,
  musicDataCache: Map<string, ParsedMusicData>,
  renderContext: PcmRenderContext,
  cryParameters?: { cryPitch: number; cryLength: number },
  includeStandardNotes = true,
): ExportedMidiAudioProgram {
  const cacheKey = `${kind}:${stem}`;
  let musicData = musicDataCache.get(cacheKey);
  if (!musicData) {
    const program = buildAsmAudioProgram(
      path.join(disassemblyRoot, "audio"),
      kind,
      stem,
    );
    if (!program) {
      throw new Error(`Audio MIDI program '${cacheKey}' has no exact ASM source.`);
    }
    musicData = new AsmAudioParser(program.source).parse();
    musicDataCache.set(cacheKey, musicData);
  }
  const rendered = includeStandardNotes
    ? new PcmConverter(
        musicData,
        renderContext.drumkits,
        renderContext.waveSamples,
        {
          qualityMode: "accurate",
          waveInstrumentMap: renderContext.waveInstrumentMap,
          loopedMusicExportSeconds: null,
          cryPitch: cryParameters?.cryPitch ?? null,
          cryLength: cryParameters?.cryLength ?? null,
          collectMidiNotes: true,
        },
      ).convert("pcm")
    : null;
  const loopStartSample = Object.entries(
    rendered?.metadata.loopSamplesByChannel ?? {},
  ).sort(([left], [right]) => Number(left) - Number(right))[0]?.[1];
  return {
    profile: "pokecrystal-midi-v1",
    midi_base64: Buffer.from(
      buildCrystalMidi(
        {
          profile: "pokecrystal-midi-v1",
          music_data: musicData,
          cry_pitch: cryParameters?.cryPitch ?? null,
          cry_length: cryParameters?.cryLength ?? null,
        },
        rendered?.midiNotes ?? [],
        loopStartSample == null
          ? null
          : {
              startSample: loopStartSample,
              endSample: rendered!.stereo.length / 2,
            },
      ),
    ).toString("base64"),
  };
}

function readCachedMidiProgram(
  asset: Pick<ExportedAudioAssetBase, "id" | "path">,
  requireStandardNotes: boolean,
): ExportedMidiAudioProgram | null {
  try {
    const metadataPath = path
      .join(getTypeScriptDataDir(), asset.path)
      .replace(/\.pcm$/, ".json");
    const payload = JSON.parse(fs.readFileSync(metadataPath, "utf8")) as Record<
      string,
      ExportedAudioAsset
    >;
    const program = payload[asset.id]?.midi_program;
    if (!program || program.profile !== "pokecrystal-midi-v1") {
      return null;
    }
    const bytes = Buffer.from(program.midi_base64, "base64");
    parseCrystalMidi(bytes);
    if (requireStandardNotes && countMidiNoteOnEvents(bytes) === 0) {
      return null;
    }
    return program;
  } catch {
    return null;
  }
}

function exportIndexedAudioPointers(
  disassemblyRoot: string,
  constantsPath: string,
  pointersPath: string,
  prefix: "MUSIC" | "SFX",
): Array<{ id: string; label: string }> {
  const constants = exportConstantsFromAsm(
    disassemblyRoot,
    constantsPath,
    prefix,
  );
  const labels = exportPointerLabelsFromAsm(disassemblyRoot, pointersPath);
  if (constants.length !== labels.length) {
    throw new Error(
      `${prefix} constants and pointer table lengths differ: ${constants.length} constants, ${labels.length} pointers.`,
    );
  }
  return constants.map((id, index) => ({ id, label: labels[index] }));
}

function requireExactCryLabel(label: unknown): string {
  if (typeof label !== "string" || !/^CRY_[A-Z0-9_]+$/.test(label)) {
    throw new Error(
      `Pokemon cry metadata must use exact CRY_* labels, got '${String(label)}'.`,
    );
  }
  return label;
}

export function exportCryLabelsFromAsm(
  disassemblyRoot = getDisassemblyRoot(),
): string[] {
  const constantsPath = path.join(
    disassemblyRoot,
    "constants",
    "cry_constants.asm",
  );
  const source = fs.readFileSync(constantsPath, "utf8");
  return uniqueSorted(
    Array.from(source.matchAll(/^\s*const\s+(CRY_[A-Z0-9_]+)\b/gm)).map(
      (match) => requireExactCryLabel(match[1]),
    ),
  );
}

export function exportPokemonCryMetadataFromAsm(
  speciesIds: Iterable<string>,
  disassemblyRoot = getDisassemblyRoot(),
): Record<string, ExportedPokemonCryMetadata> {
  const speciesSet = new Set(speciesIds);
  const sourcePath = path.join(disassemblyRoot, "data", "pokemon", "cries.asm");
  const source = fs.readFileSync(sourcePath, "utf8");
  const pokemonCries: Record<string, ExportedPokemonCryMetadata> = {};

  for (const line of source.split(/\r?\n/)) {
    const match = line.match(
      /^\s*mon_cry\s+([^,]+),\s*([^,]+),\s*([^;]+);\s*([A-Z0-9_]+)\s*$/,
    );
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
      throw new Error(
        `Pokemon cry metadata is missing exact species '${speciesId}'.`,
      );
    }
  }

  return pokemonCries;
}

function parseExactInteger(value: string, speciesId: string): number {
  if (!/^-?\d+$/.test(value)) {
    throw new Error(
      `Pokemon cry metadata for '${speciesId}' has non-integer value '${value}'.`,
    );
  }
  return Number(value);
}

export function exportAudioAssets(
  pokemonCries: Record<string, ExportedPokemonCryMetadata>,
  disassemblyRoot = getDisassemblyRoot(),
): Record<string, ExportedAudioAsset> {
  for (const cry of Object.values(pokemonCries)) {
    requireExactCryLabel(cry.cry);
  }
  const midiSourceCache = new Map<string, ParsedMusicData>();
  const renderContext = loadPcmRenderContext(path.join(disassemblyRoot, "audio"));
  const cacheManifest = expectedAudioCacheManifest(disassemblyRoot);
  const reuseCache = cacheManifestMatches(cacheManifest);
  const musicLabelStems = exportMusicLabelStemsFromAsm(disassemblyRoot);
  const music = exportIndexedAudioPointers(
    disassemblyRoot,
    "constants/music_constants.asm",
    "audio/music_pointers.asm",
    "MUSIC",
  ).filter(({ id }) => id !== "MUSIC_NONE").map(({ id, label }) => {
    const stem = musicLabelStems.get(label);
    if (!stem) {
      throw new Error(
        `Music pointer '${label}' for '${id}' does not match an exact audio/music ASM label.`,
      );
    }
    const asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata | "midi_program"> = {
      id,
      path: `${CORE_AUDIO_ROOT}/music/${id}.pcm`,
      kind: "music",
      source: "pcm",
      pcm_format: PCM_AUDIO_FORMAT,
    };
    const metadata = writeRenderedPcmAsset(
      asset,
      disassemblyRoot,
      stem,
      undefined,
      reuseCache,
    );
    return {
      ...asset,
      ...metadata,
      midi_program: (reuseCache && readCachedMidiProgram(asset, true)) || buildMidiAudioProgram(
        disassemblyRoot,
        "music",
        stem,
        midiSourceCache,
        renderContext,
      ),
    } satisfies RenderedPcmAudioAsset;
  });

  const sfx = exportIndexedAudioPointers(
    disassemblyRoot,
    "constants/sfx_constants.asm",
    "audio/sfx_pointers.asm",
    "SFX",
  ).map(({ id, label }, sfxPriority) => {
    if (sfxPriority > 0xff) {
      throw new Error(
        `SFX priority for '${id}' exceeds Crystal's one-byte wCurSFX boundary.`,
      );
    }
    if (!label.startsWith("Sfx_")) {
      throw new Error(
        `SFX pointer '${label}' for '${id}' must use an exact Sfx_* label.`,
      );
    }
    const asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata | "midi_program"> = {
      id,
      path: `${CORE_AUDIO_ROOT}/sfx/${id}.pcm`,
      kind: "sound_effect",
      sfx_priority: sfxPriority,
      source: "pcm",
      pcm_format: PCM_AUDIO_FORMAT,
    };
    const metadata = writeRenderedPcmAsset(
      asset,
      disassemblyRoot,
      label.replace(/^Sfx_/, ""),
      undefined,
      reuseCache,
    );
    return {
      ...asset,
      ...metadata,
      midi_program: (reuseCache && readCachedMidiProgram(asset, true)) || buildMidiAudioProgram(
        disassemblyRoot,
        "sfx",
        label.replace(/^Sfx_/, ""),
        midiSourceCache,
        renderContext,
      ),
    } satisfies RenderedPcmAudioAsset;
  });

  const cries = uniqueSorted([
    ...Object.values(pokemonCries).map((cry) => requireExactCryLabel(cry.cry)),
    ...exportCryLabelsFromAsm(),
  ]).map((label) => {
    const asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata | "midi_program"> = {
      id: label,
      path: `${CORE_AUDIO_ROOT}/cries/${label}.pcm`,
      kind: "cry",
      source: "pcm",
      pcm_format: PCM_AUDIO_FORMAT,
    };
    const metadata = writeRenderedPcmAsset(
      asset,
      disassemblyRoot,
      label.replace(/^CRY_/, ""),
      undefined,
      reuseCache,
    );
    return {
      ...asset,
      ...metadata,
      midi_program: (reuseCache && readCachedMidiProgram(asset, true)) || buildMidiAudioProgram(
        disassemblyRoot,
        "cry",
        label.replace(/^CRY_/, ""),
        midiSourceCache,
        renderContext,
      ),
    } satisfies RenderedPcmAudioAsset;
  });

  const speciesCries = Object.entries(pokemonCries)
    .sort(([left], [right]) => left.localeCompare(right))
    .flatMap(([speciesId, cry]) => {
      const stem = requireExactCryLabel(cry.cry).replace(/^CRY_/, "");
      return pokemonCryVariantIds(speciesId).map((id, index) => {
        const lengthOffset = [0, 0x00c0, 0x0040][index];
        const asset: Omit<RenderedPcmAudioAsset, keyof PcmPayloadMetadata | "midi_program"> = {
          id,
          path: `${CORE_AUDIO_ROOT}/cries/${id}.pcm`,
          kind: "cry",
          source: "pcm",
          pcm_format: PCM_AUDIO_FORMAT,
        };
        const metadata = writeRenderedPcmAsset(
          asset,
          disassemblyRoot,
          stem,
          {
            cryPitch: cry.pitch,
            cryLength: cry.length + lengthOffset,
          },
          reuseCache,
        );
        return {
          ...asset,
          ...metadata,
          midi_program: (reuseCache && readCachedMidiProgram(asset, false)) || buildMidiAudioProgram(
            disassemblyRoot,
            "cry",
            stem,
            midiSourceCache,
            renderContext,
            {
              cryPitch: cry.pitch,
              cryLength: cry.length + lengthOffset,
            },
            false,
          ),
        } satisfies RenderedPcmAudioAsset;
      });
    });

  const assets: Record<string, ExportedAudioAsset> = {};
  for (const asset of [...music, ...sfx, ...cries, ...speciesCries]) {
    if (Object.prototype.hasOwnProperty.call(assets, asset.id)) {
      throw new Error(`duplicate audio asset id ${asset.id}`);
    }
    writeJsonToTargets(
      asset.path.replace(/\.pcm$/, ".json"),
      { [asset.id]: asset },
      { indent: 2 },
    );
    fs.writeFileSync(
      path.join(getTypeScriptDataDir(), asset.path.replace(/\.pcm$/, ".mid")),
      Buffer.from(asset.midi_program!.midi_base64, "base64"),
    );
    assets[asset.id] = asset;
  }
  writeJsonToTargets(
    `${CORE_AUDIO_ROOT}/audio-synth-context.json`,
    renderContext,
    { indent: 0 },
  );
  fs.rmSync(
    path.join(getTypeScriptDataDir(), CORE_AUDIO_ROOT, "music", "MUSIC_NONE.pcm"),
    { force: true },
  );
  ensureDir(path.dirname(audioCachePath()));
  fs.writeFileSync(
    audioCachePath(),
    `${JSON.stringify(cacheManifest, null, 2)}\n`,
  );
  return assets;
}
