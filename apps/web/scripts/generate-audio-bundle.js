#!/usr/bin/env node

const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const Module = require("node:module");
const { spawnSync } = require("node:child_process");
const ts = require("typescript");

const projectRoot = path.resolve(__dirname, "..");
const repoRoot = path.resolve(projectRoot, "..", "..");
const assetsAudioRoot = path.join(projectRoot, "assets", "audio");
const disassemblyRoot = process.env.POKECRYSTAL_DISASSEMBLY_ROOT
  ? path.resolve(process.env.POKECRYSTAL_DISASSEMBLY_ROOT)
  : path.join(repoRoot, "vendor", "pokecrystal");
const disassemblyAudioRoot = path.join(disassemblyRoot, "audio");
const EXPORT_INFINITE_LOOP_REPEAT_LIMIT = 2;

const LEGACY_SLUG_OVERRIDES = new Map([
  ["Nidoran_M", "nidoran_m"],
  ["Nidoran_F", "nidoran_f"],
  ["Unknown5F", "unused"],
]);

const normalizeAsmSlug = (value) => LEGACY_SLUG_OVERRIDES.get(value) ?? value.replace(/[^A-Za-z0-9]/g, "").toLowerCase();
const normalizeStandaloneLocalLabels = (sourceText) =>
  sourceText.replace(/^(\s*)(\.[A-Za-z0-9_]+)\s*$/gm, "$1$2:");
const isPerChannelLabel = (label) => /_Ch\d+$/.test(label);

const buildMusicOutputPath = (basename, outputRoot = assetsAudioRoot) =>
  path.join(outputRoot, `${basename}.mp3`);

const buildEffectOutputPath = (group, slug, outputRoot = assetsAudioRoot) =>
  path.join(outputRoot, group, `${slug}.mp3`);

const buildManifestOutputPath = (group, slug, outputRoot = assetsAudioRoot) =>
  path.join(outputRoot, "manifests", group, `${slug}.json`);

const buildMusicStemOutputPath = (basename, channel, outputRoot = assetsAudioRoot) =>
  path.join(outputRoot, "music", "stems", basename, `ch${channel}.mp3`);

const buildRuntimeManifestOutputPath = (outputRoot = assetsAudioRoot) =>
  path.join(outputRoot, "manifests", "runtime.json");

const assertDisassemblyAudioSource = (sourceRoot) => {
  const required = [
    path.join(sourceRoot, "music"),
    path.join(sourceRoot, "sfx.asm"),
    path.join(sourceRoot, "sfx_crystal.asm"),
    path.join(sourceRoot, "cries.asm"),
    path.join(sourceRoot, "drumkits.asm"),
    path.join(sourceRoot, "wave_samples.asm"),
  ];
  const missing = required.filter((candidate) => !fs.existsSync(candidate));
  if (missing.length > 0) {
    throw new Error(
      [
        "Audio bundle generation requires compiled pret/pokecrystal audio ASM sources.",
        `Expected them under: ${sourceRoot}`,
        "Set POKECRYSTAL_DISASSEMBLY_ROOT to a complete disassembly checkout if needed.",
        `Missing: ${missing.map((candidate) => path.relative(sourceRoot, candidate)).join(", ")}`,
      ].join("\n"),
    );
  }
};

const inferPriorityClass = (token, kind) => {
  if (kind === "cries") {
    return "cry";
  }
  const normalized = String(token ?? "").trim().toUpperCase();
  if (!normalized.startsWith("SFX_")) {
    return "none";
  }
  if (
    normalized.startsWith("SFX_DEX_FANFARE_") ||
    normalized.startsWith("SFX_GET_") ||
    [
      "SFX_FANFARE",
      "SFX_FANFARE_2",
      "SFX_CAUGHT_MON",
      "SFX_LEVEL_UP",
      "SFX_REGISTER_PHONE_NUMBER",
      "SFX_PRESENT",
      "SFX_1ST_PLACE",
      "SFX_2ND_PLACE",
      "SFX_3RD_PLACE",
      "SFX_EVOLVED",
    ].includes(normalized)
  ) {
    return "priority";
  }
  return "none";
};

const listTopLevelLabels = (sourceText, prefix) => {
  const labels = [];
  for (const match of sourceText.matchAll(/^([A-Za-z0-9_]+):\s*$/gm)) {
    if (match[1].startsWith(prefix)) {
      labels.push(match[1]);
    }
  }
  return labels;
};

const extractAsmProgram = (sourceText, entryLabel) => {
  const lines = normalizeStandaloneLocalLabels(sourceText).split(/\r?\n/);
  const labelIndex = new Map();
  for (let i = 0; i < lines.length; i += 1) {
    const match = lines[i].match(/^([A-Za-z0-9_.]+):\s*$/);
    if (match) {
      labelIndex.set(match[1], i);
    }
  }

  const readBlock = (label) => {
    const start = labelIndex.get(label);
    if (start === undefined) {
      return null;
    }
    let end = lines.length;
    for (let i = start + 1; i < lines.length; i += 1) {
      if (/^[A-Za-z0-9_]+:\s*$/.test(lines[i])) {
        end = i;
        break;
      }
    }
    return lines.slice(start, end);
  };

  const queue = [entryLabel];
  const seen = new Set();
  const blocks = [];

  while (queue.length > 0) {
    const label = queue.shift();
    if (!label || seen.has(label)) {
      continue;
    }
    seen.add(label);
    const block = readBlock(label);
    if (!block) {
      continue;
    }
    const blockText = block.join("\n");
    blocks.push(blockText);
    const owner = label.startsWith(".") ? null : label;

    for (const match of blockText.matchAll(/^\s*channel\s+\d+\s*,\s*([A-Za-z0-9_.]+)/gm)) {
      queue.push(match[1]);
    }

    for (const match of blockText.matchAll(/^\s*sound_call\s+([A-Za-z0-9_.]+)/gm)) {
      const raw = match[1];
      queue.push(raw.startsWith(".") && owner ? `${owner}${raw}` : raw);
    }
  }

  return blocks.length > 0 ? blocks.join("\n\n") : null;
};

const createWavFromStereo16 = (interleavedStereo, sampleRate) => {
  const channels = 2;
  const bitsPerSample = 16;
  const byteRate = sampleRate * channels * (bitsPerSample / 8);
  const blockAlign = channels * (bitsPerSample / 8);
  const dataBytes = interleavedStereo.length * 2;
  const buffer = new ArrayBuffer(44 + dataBytes);
  const view = new DataView(buffer);

  writeAscii(view, 0, "RIFF");
  view.setUint32(4, 36 + dataBytes, true);
  writeAscii(view, 8, "WAVE");
  writeAscii(view, 12, "fmt ");
  view.setUint32(16, 16, true);
  view.setUint16(20, 1, true);
  view.setUint16(22, channels, true);
  view.setUint32(24, sampleRate, true);
  view.setUint32(28, byteRate, true);
  view.setUint16(32, blockAlign, true);
  view.setUint16(34, bitsPerSample, true);
  writeAscii(view, 36, "data");
  view.setUint32(40, dataBytes, true);

  let offset = 44;
  for (let i = 0; i < interleavedStereo.length; i += 1) {
    view.setInt16(offset, interleavedStereo[i], true);
    offset += 2;
  }

  return new Uint8Array(buffer);
};

const padStereoForMp3Encoding = (interleavedStereo, sampleRate, minimumDurationSeconds = 0.1) => {
  const minimumFrames = Math.max(1, Math.ceil(sampleRate * minimumDurationSeconds));
  const minimumSamples = minimumFrames * 2;
  if (interleavedStereo.length >= minimumSamples) {
    return interleavedStereo;
  }
  const padded = new Int16Array(minimumSamples);
  padded.set(interleavedStereo, 0);
  return padded;
};

const writeAscii = (view, offset, text) => {
  for (let i = 0; i < text.length; i += 1) {
    view.setUint8(offset + i, text.charCodeAt(i));
  }
};

const ensureTypeScriptRuntime = () => {
  if (global.__POKECRYSTAL_TS_RUNTIME_READY__) {
    return;
  }
  const originalResolve = Module._resolveFilename;
  Module._resolveFilename = function patchedResolve(request, parent, isMain, options) {
    if (request.startsWith("@/")) {
      request = path.join(projectRoot, "src", request.slice(2));
    } else if (request.startsWith("@pokecrystal/core/")) {
      request = path.join(repoRoot, "packages", "core", "src", request.slice("@pokecrystal/core/".length));
    } else if (request === "@pokecrystal/core") {
      request = path.join(repoRoot, "packages", "core", "src", "index.ts");
    }
    return originalResolve.call(this, request, parent, isMain, options);
  };

  require.extensions[".ts"] = function registerTs(module, filename) {
    const source = fs.readFileSync(filename, "utf8");
    const { outputText } = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2020,
        esModuleInterop: true,
        resolveJsonModule: true,
      },
      fileName: filename,
    });
    module._compile(outputText, filename);
  };

  global.__POKECRYSTAL_TS_RUNTIME_READY__ = true;
};

const collectJobs = (sourceRoot = disassemblyAudioRoot, outputRoot = assetsAudioRoot) => {
  assertDisassemblyAudioSource(sourceRoot);
  const jobs = [];
  const musicDir = path.join(sourceRoot, "music");
  for (const entry of fs.readdirSync(musicDir, { withFileTypes: true })) {
    if (!entry.isFile() || path.extname(entry.name) !== ".asm") {
      continue;
    }
    const basename = path.basename(entry.name, ".asm");
    const sourcePath = path.join(musicDir, entry.name);
    jobs.push({
      label: basename,
      kind: "music",
      sourcePath,
      asmText: normalizeStandaloneLocalLabels(fs.readFileSync(sourcePath, "utf8")),
      outputPath: buildMusicOutputPath(basename, outputRoot),
    });
  }

  for (const [fileName, prefix, group] of [
    ["sfx.asm", "Sfx_", "sfx"],
    ["sfx_crystal.asm", "Sfx_", "sfx"],
    ["cries.asm", "Cry_", "cries"],
  ]) {
    const sourcePath = path.join(sourceRoot, fileName);
    const sourceText = normalizeStandaloneLocalLabels(fs.readFileSync(sourcePath, "utf8"));
    for (const label of listTopLevelLabels(sourceText, prefix)) {
      if (isPerChannelLabel(label)) {
        continue;
      }
      const asmText = extractAsmProgram(sourceText, label);
      if (!asmText) {
        continue;
      }
      const slug = normalizeAsmSlug(label.replace(prefix, ""));
      jobs.push({
        label,
        kind: group,
        sourcePath,
        asmText,
        outputPath: buildEffectOutputPath(group, slug, outputRoot),
      });
    }
  }

  return jobs;
};

const loadAudioToolchain = () => {
  ensureTypeScriptRuntime();
  const { AsmAudioParser, DrumkitParser, WaveSampleParser } = require(path.join(repoRoot, "packages", "core", "src", "audio-export", "parsers.ts"));
  const { WavConverter } = require(path.join(repoRoot, "packages", "core", "src", "audio-export", "converter.ts"));
  return { AsmAudioParser, DrumkitParser, WaveSampleParser, WavConverter };
};

const encodeMp3 = (wavBytes, outputPath) => {
  const tempBase = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-audio-"));
  const wavPath = path.join(tempBase, "input.wav");
  fs.writeFileSync(wavPath, Buffer.from(wavBytes));
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  const result = spawnSync(
    "ffmpeg",
    ["-y", "-loglevel", "error", "-i", wavPath, "-codec:a", "libmp3lame", "-b:a", "64k", outputPath],
    { stdio: "pipe" },
  );
  fs.rmSync(tempBase, { recursive: true, force: true });
  if (result.status !== 0) {
    throw new Error(`ffmpeg failed for ${outputPath}: ${result.stderr.toString("utf8")}`);
  }
};

const writeJson = (outputPath, payload) => {
  fs.mkdirSync(path.dirname(outputPath), { recursive: true });
  fs.writeFileSync(outputPath, JSON.stringify(payload, null, 2));
};

const renderPcmResult = (converter, outputPath) => {
  const result = converter.convert("pcm");
  const wavBytes = createWavFromStereo16(
    padStereoForMp3Encoding(result.stereo, result.sampleRate),
    result.sampleRate,
  );
  encodeMp3(wavBytes, outputPath);
  return result;
};

const generateBundle = ({ force = false, sourceRoot = disassemblyAudioRoot, outputRoot = assetsAudioRoot } = {}) => {
  assertDisassemblyAudioSource(sourceRoot);
  const { AsmAudioParser, DrumkitParser, WaveSampleParser, WavConverter } = loadAudioToolchain();
  const drumkitsText = fs.readFileSync(path.join(sourceRoot, "drumkits.asm"), "utf8");
  const waveSamplesText = fs.readFileSync(path.join(sourceRoot, "wave_samples.asm"), "utf8");
  const drumkits = new DrumkitParser().parseFromText(drumkitsText);
  const waveSampleParser = new WaveSampleParser();
  const waveSamples = waveSampleParser.parseFromText(waveSamplesText);
  const jobs = collectJobs(sourceRoot, outputRoot);
  const runtimeManifest = { music: {}, sounds: {} };

  let generated = 0;
  let skipped = 0;
  for (const [index, job] of jobs.entries()) {
    const slug = path.basename(job.outputPath, ".mp3");
    const manifestPath = buildManifestOutputPath(job.kind, slug, outputRoot);
    const hasOutputs = fs.existsSync(job.outputPath) && fs.existsSync(manifestPath);
    if (!force && hasOutputs) {
      if (job.kind === "music") {
        runtimeManifest.music[job.label] = path.relative(outputRoot, manifestPath).replace(/\\/g, "/");
      } else {
        runtimeManifest.sounds[job.label] = path.relative(outputRoot, manifestPath).replace(/\\/g, "/");
      }
      skipped += 1;
      continue;
    }
    const musicData = new AsmAudioParser(job.asmText).parse();
    const converter = new WavConverter(
      musicData,
      drumkits,
      waveSamples,
      {
        waveInstrumentMap: waveSampleParser.instrumentMap,
        infiniteLoopRepeatLimit: EXPORT_INFINITE_LOOP_REPEAT_LIMIT,
      },
    );
    const result = renderPcmResult(converter, job.outputPath);
    if (job.kind === "music") {
      const stems = [];
      for (const [channelLabel, channelData] of Object.entries(musicData.channels)) {
        const channel = channelData.number ?? stems.length + 1;
        const stemOutputPath = buildMusicStemOutputPath(slug, channel, outputRoot);
        const stemConverter = new WavConverter(
          musicData,
          drumkits,
          waveSamples,
          {
            waveInstrumentMap: waveSampleParser.instrumentMap,
            infiniteLoopRepeatLimit: EXPORT_INFINITE_LOOP_REPEAT_LIMIT,
            soloChannel: channel,
          },
        );
        renderPcmResult(stemConverter, stemOutputPath);
        stems.push({
          channel,
          path: `music/stems/${slug}/ch${channel}.mp3`,
          loop: true,
          pan: [true, true],
        });
      }
      writeJson(manifestPath, {
        kind: "music",
        token: job.label,
        mixedPath: `${slug}.mp3`,
        channelCount: musicData.channel_count,
        loop: true,
        loopStartFrame: result.metadata.loopFramesByChannel?.[1] ?? null,
        loopStartSeconds:
          typeof result.metadata.loopSamplesByChannel?.[1] === "number"
            ? result.metadata.loopSamplesByChannel[1] / result.sampleRate
            : null,
        stems,
      });
      runtimeManifest.music[job.label] = path.relative(outputRoot, manifestPath).replace(/\\/g, "/");
    } else {
      const ownedChannels = Object.values(musicData.channels)
        .map((entry) => entry.number)
        .filter((value) => typeof value === "number");
      writeJson(manifestPath, {
        kind: job.kind === "cries" ? "cry" : "sfx",
        token: job.label,
        assetPath: path.relative(outputRoot, job.outputPath).replace(/\\/g, "/"),
        ownedChannels,
        durationFrames:
          typeof result.metadata.durationSeconds === "number"
            ? Math.max(1, Math.round((result.metadata.durationSeconds * 1000) / 16.74))
            : null,
        priorityClass: inferPriorityClass(job.label, job.kind),
      });
      runtimeManifest.sounds[job.label] = path.relative(outputRoot, manifestPath).replace(/\\/g, "/");
    }
    generated += 1;
    if ((index + 1) % 25 === 0 || index === jobs.length - 1) {
      console.log(`[audio-bundle] ${index + 1}/${jobs.length} processed`);
    }
  }

  writeJson(buildRuntimeManifestOutputPath(outputRoot), runtimeManifest);

  return { generated, skipped, total: jobs.length };
};

if (require.main === module) {
  const force = process.argv.includes("--force");
  const result = generateBundle({ force });
  console.log(
    `[audio-bundle] generated=${result.generated} skipped=${result.skipped} total=${result.total}`,
  );
}

module.exports = {
  normalizeAsmSlug,
  normalizeStandaloneLocalLabels,
  isPerChannelLabel,
  buildMusicOutputPath,
  buildEffectOutputPath,
  listTopLevelLabels,
  extractAsmProgram,
  createWavFromStereo16,
  padStereoForMp3Encoding,
  buildManifestOutputPath,
  buildMusicStemOutputPath,
  buildRuntimeManifestOutputPath,
  inferPriorityClass,
  collectJobs,
  generateBundle,
};
