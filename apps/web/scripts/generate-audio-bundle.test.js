const {
  isPerChannelLabel,
  normalizeAsmSlug,
  normalizeStandaloneLocalLabels,
  buildMusicOutputPath,
  buildEffectOutputPath,
  listTopLevelLabels,
  extractAsmProgram,
  collectJobs,
  createWavFromStereo16,
  padStereoForMp3Encoding,
  buildManifestOutputPath,
  buildMusicStemOutputPath,
  buildRuntimeManifestOutputPath,
  inferPriorityClass,
} = require("./generate-audio-bundle");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const makeDir = (dir) => fs.mkdirSync(dir, { recursive: true });

describe("generate-audio-bundle", () => {
  it("builds output paths from explicit roots", () => {
    const outputRoot = "/tmp/audio-root";
    expect(buildMusicOutputPath("route29", outputRoot)).toBe(path.join(outputRoot, "route29.mp3"));
    expect(buildEffectOutputPath("sfx", "menu-open", outputRoot)).toBe(
      path.join(outputRoot, "sfx", "menu-open.mp3"),
    );
    expect(buildEffectOutputPath("cries", "pikachu", outputRoot)).toBe(
      path.join(outputRoot, "cries", "pikachu.mp3"),
    );
    expect(buildManifestOutputPath("music", "route29", outputRoot)).toBe(
      path.join(outputRoot, "manifests", "music", "route29.json"),
    );
    expect(buildMusicStemOutputPath("route29", 2, outputRoot)).toBe(
      path.join(outputRoot, "music", "stems", "route29", "ch2.mp3"),
    );
    expect(buildRuntimeManifestOutputPath(outputRoot)).toBe(
      path.join(outputRoot, "manifests", "runtime.json"),
    );
  });

  it("finds top-level labels for a given prefix", () => {
    const sourceText = ["Sfx_Menu:", "  channel 5, Sfx_Menu_Ch5", "Sfx_Menu_Ch5:", "Music_Test:", "Sfx_Blast: "].join(
      "\n",
    );
    expect(listTopLevelLabels(sourceText, "Sfx_")).toEqual(["Sfx_Menu", "Sfx_Menu_Ch5", "Sfx_Blast"]);
  });

  it("returns null for missing extraction targets", () => {
    const asm = ["Music_Test:", "  note C_, 4"].join("\n");
    expect(extractAsmProgram(asm, "Missing")).toBeNull();
  });

  it("extracts a label and its channel and sound_call dependencies", () => {
    const asm = [
      "Sfx_Test:",
      "\tchannel 5, Sfx_Test_Ch5",
      "Sfx_Test_Ch5:",
      "\tsound_call .branch",
      "\tsound_ret",
      "Sfx_Test.branch:",
      "\tsquare_note 4, 15, 0, 1024",
      "\tsound_ret",
      "Other_Label:",
      "\tsound_ret",
    ].join("\n");

    const extracted = extractAsmProgram(asm, "Sfx_Test");

    expect(extracted).toContain("Sfx_Test:");
    expect(extracted).toContain("Sfx_Test_Ch5:");
    expect(extracted).toContain("Sfx_Test.branch:");
    expect(extracted).not.toContain("Other_Label:");
  });

  it("filters per-channel labels from standalone asset generation", () => {
    expect(isPerChannelLabel("Cry_Entei_Ch5")).toBe(true);
    expect(isPerChannelLabel("Sfx_Save_Ch6")).toBe(true);
    expect(isPerChannelLabel("Cry_Entei")).toBe(false);
  });

  it("writes a valid WAV RIFF header", () => {
    const wavBytes = createWavFromStereo16(new Int16Array([0, 1234]), 22050);
    const chunkId = String.fromCharCode(wavBytes[0], wavBytes[1], wavBytes[2], wavBytes[3]);
    const waveId = String.fromCharCode(wavBytes[8], wavBytes[9], wavBytes[10], wavBytes[11]);
    const fmtId = String.fromCharCode(wavBytes[12], wavBytes[13], wavBytes[14], wavBytes[15]);
    expect(chunkId).toBe("RIFF");
    expect(waveId).toBe("WAVE");
    expect(fmtId).toBe("fmt ");
    expect(new DataView(wavBytes.buffer).getUint16(34, true)).toBe(16);
  });

  it("pads very short stereo clips before MP3 encoding", () => {
    const stereo = new Int16Array([1, -1, 2, -2]);
    const padded = padStereoForMp3Encoding(stereo, 44100, 0.1);
    expect(padded.length).toBe(4410 * 2);
    expect(Array.from(padded.slice(0, stereo.length))).toEqual(Array.from(stereo));
  });

  it("preserves legacy slug names that differ from naive normalization", () => {
    expect(normalizeAsmSlug("Nidoran_M")).toBe("nidoran_m");
    expect(normalizeAsmSlug("Nidoran_F")).toBe("nidoran_f");
    expect(normalizeAsmSlug("Unknown5F")).toBe("unused");
  });

  it("normalizes local labels without trailing colon", () => {
    const normalized = normalizeStandaloneLocalLabels("Sfx_Test:\n.loop\n\tnote C_, 4\n");
    expect(normalized).toContain(".loop:");
  });

  it("marks cries and congratulatory fanfares as priority manifests", () => {
    expect(inferPriorityClass("Cry_Wooper", "cries")).toBe("cry");
    expect(inferPriorityClass("SFX_CAUGHT_MON", "sfx")).toBe("priority");
    expect(inferPriorityClass("SFX_DEX_FANFARE_50_79", "sfx")).toBe("priority");
    expect(inferPriorityClass("SFX_ITEM", "sfx")).toBe("none");
  });

  it("collects music, sfx, and cry bundle jobs from ASM assets", () => {
    const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-audio-jobs-"));

    try {
      const sourceRoot = path.join(tempRoot, "audio");
      const outputRoot = path.join(tempRoot, "out");
      makeDir(path.join(sourceRoot, "music"));

      fs.writeFileSync(path.join(sourceRoot, "music", "route29.asm"), "Music_Route29:\n\tsound_ret\n");
      fs.writeFileSync(
        path.join(sourceRoot, "sfx.asm"),
        ["Sfx_Menu:", "\tchannel 5, Sfx_Menu_Ch5", "Sfx_Menu_Ch5:", "\tsound_ret"].join("\n"),
      );
      fs.writeFileSync(path.join(sourceRoot, "sfx_crystal.asm"), "");
      fs.writeFileSync(path.join(sourceRoot, "drumkits.asm"), "");
      fs.writeFileSync(path.join(sourceRoot, "wave_samples.asm"), "");
      fs.writeFileSync(
        path.join(sourceRoot, "cries.asm"),
        ["Cry_Pikachu:", "\tchannel 8, Cry_Pikachu_Ch8", "Cry_Pikachu_Ch8:", "\tsound_ret"].join("\n"),
      );

      const jobs = collectJobs(sourceRoot, outputRoot);
      expect(jobs.find((job) => job.label === "Sfx_Menu_Ch5")).toBeUndefined();

      expect(jobs).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            kind: "music",
            label: "route29",
            outputPath: path.join(outputRoot, "route29.mp3"),
          }),
          expect.objectContaining({
            kind: "sfx",
            label: "Sfx_Menu",
            outputPath: path.join(outputRoot, "sfx", "menu.mp3"),
          }),
          expect.objectContaining({
            kind: "cries",
            label: "Cry_Pikachu",
            outputPath: path.join(outputRoot, "cries", "pikachu.mp3"),
          }),
        ]),
      );
    } finally {
      fs.rmSync(tempRoot, { recursive: true, force: true });
    }
  });
});
