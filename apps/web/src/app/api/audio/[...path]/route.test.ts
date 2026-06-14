import fs from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import {
  GET,
  buildAudioEtag,
  isFreshAudioRequest,
} from "./route";

describe("audio route cache helpers", () => {
  it("builds a deterministic etag from mtime and size", () => {
    const etag = buildAudioEtag({ mtimeMs: 12345.67, size: 4096 });
    expect(etag).toBe('"3039-1000"');
  });

  it("detects fresh requests via if-none-match", () => {
    const request = new Request("https://example.com/api/audio/test.mp3", {
      headers: {
        "if-none-match": '"etag-123"',
      },
    });

    expect(isFreshAudioRequest(request, '"etag-123"')).toBe(true);
    expect(isFreshAudioRequest(request, '"etag-456"')).toBe(false);
  });
});

describe("audio route asset resolution", () => {
  const fixtureDir = path.resolve(process.cwd(), "public", "assets", "audio", "__tests__");
  const fixturePath = path.join(fixtureDir, "fixture.wav");
  const manifestPath = path.join(fixtureDir, "fixture.json");
  const musicDir = path.resolve(process.cwd(), "public", "assets", "audio", "music");
  const missingMusicAsmPath = path.join(musicDir, "missingbundledtrack.asm");
  let tempDisassemblyRoot: string | null = null;
  let originalDisassemblyRoot: string | undefined;

  beforeEach(async () => {
    originalDisassemblyRoot = process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
    tempDisassemblyRoot = await fs.mkdtemp(path.join(os.tmpdir(), "pokecrystal-audio-disassembly-"));
    const tempAudioRoot = path.join(tempDisassemblyRoot, "audio");
    await fs.mkdir(fixtureDir, { recursive: true });
    await fs.mkdir(musicDir, { recursive: true });
    await fs.mkdir(tempAudioRoot, { recursive: true });
    await fs.mkdir(path.join(tempAudioRoot, "music"), { recursive: true });
    process.env.POKECRYSTAL_DISASSEMBLY_ROOT = tempDisassemblyRoot;
    await fs.writeFile(
      fixturePath,
      Buffer.from([
        0x52, 0x49, 0x46, 0x46, 0x24, 0x00, 0x00, 0x00,
        0x57, 0x41, 0x56, 0x45, 0x66, 0x6d, 0x74, 0x20,
        0x10, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00,
        0x22, 0x56, 0x00, 0x00, 0x88, 0x58, 0x01, 0x00,
        0x04, 0x00, 0x10, 0x00, 0x64, 0x61, 0x74, 0x61,
        0x00, 0x00, 0x00, 0x00,
      ]),
    );
    await fs.writeFile(
      manifestPath,
      JSON.stringify({ ok: true }),
    );
    await fs.writeFile(
      missingMusicAsmPath,
      [
        "Music_MissingBundledTrack:",
        "\tchannel 1, Music_MissingBundledTrack_Ch1",
        "Music_MissingBundledTrack_Ch1:",
        "\tnote_type 12, 15, 0",
        "\tnote C_, 4",
        "\tsound_ret",
      ].join("\n"),
    );
    await fs.writeFile(
      path.join(tempAudioRoot, "music", "testpcm.asm"),
      [
        "Music_TestPcm:",
        "\tchannel 1, Music_TestPcm_Ch1",
        "\tchannel 2, Music_TestPcm_Ch2",
        "",
        "Music_TestPcm_Ch1:",
        "\tnote_type 12, 15, 0",
        "\toctave 4",
        ".mainloop:",
        "\tnote C_, 4",
        "\tsound_loop 0, .mainloop",
        "",
        "Music_TestPcm_Ch2:",
        "\tnote_type 12, 12, 0",
        "\toctave 4",
        ".mainloop:",
        "\tnote E_, 4",
        "\tsound_loop 0, .mainloop",
      ].join("\n"),
    );
    await fs.writeFile(
      path.join(tempAudioRoot, "sfx.asm"),
      [
        "Sfx_TestSynth:",
        "\tchannel_count 1",
        "\tchannel 8, Sfx_TestSynth_Ch8",
        "",
        "Sfx_TestSynth_Ch8:",
        "\tnoise_note 1, 14, 2, 51",
        "\tnoise_note 8, 14, 1, 34",
        "\tsound_ret",
      ].join("\n"),
    );
    await fs.writeFile(path.join(tempAudioRoot, "sfx_crystal.asm"), "");
    await fs.writeFile(
      path.join(tempAudioRoot, "cries.asm"),
      [
        "Cry_Nidoran_M:",
        "\tchannel_count 1",
        "\tchannel 5, Cry_Nidoran_M_Ch5",
        "",
        "Cry_Nidoran_M_Ch5:",
        "\tduty_cycle_pattern 0, 1, 2, 3",
        "\tsquare_note 4, 15, 2, 1792",
        "\tsound_ret",
      ].join("\n"),
    );
    await fs.writeFile(path.join(tempAudioRoot, "drumkits.asm"), "");
    await fs.writeFile(path.join(tempAudioRoot, "wave_samples.asm"), "");
  });

  afterEach(async () => {
    await fs.rm(fixtureDir, { recursive: true, force: true });
    await fs.rm(missingMusicAsmPath, { force: true });
    if (tempDisassemblyRoot) {
      await fs.rm(tempDisassemblyRoot, { recursive: true, force: true });
    }
    if (originalDisassemblyRoot === undefined) {
      delete process.env.POKECRYSTAL_DISASSEMBLY_ROOT;
    } else {
      process.env.POKECRYSTAL_DISASSEMBLY_ROOT = originalDisassemblyRoot;
    }
  });

  it("streams deployable bundled audio files from public assets", async () => {
    const response = await GET(
      new Request("https://example.com/api/audio/__tests__/fixture.wav"),
      { params: Promise.resolve({ path: ["__tests__", "fixture.wav"] }) },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Type")).toBe("audio/wav");
    expect(response.headers.get("Cache-Control")).toBe("public, max-age=31536000, immutable");
    expect((await response.arrayBuffer()).byteLength).toBe(44);
  });

  it("streams bundled audio manifests as json", async () => {
    const response = await GET(
      new Request("https://example.com/api/audio/__tests__/fixture.json"),
      { params: Promise.resolve({ path: ["__tests__", "fixture.json"] }) },
    );

    expect(response.status).toBe(200);
    expect(response.headers.get("Content-Type")).toContain("application/json");
    expect(await response.json()).toEqual({ ok: true });
  });

  it("does not synthesize legacy mp3 media from the configured disassembly", async () => {
    const response = await GET(
      new Request("https://example.com/api/audio/sfx/testsynth.mp3"),
      { params: Promise.resolve({ path: ["sfx", "testsynth.mp3"] }) },
    );

    expect(response.status).toBe(404);
  });

  it("serves direct PCM music manifests and raw channel data", async () => {
    const manifestResponse = await GET(
      new Request("https://example.com/api/audio/pcm/music/testpcm.json"),
      { params: Promise.resolve({ path: ["pcm", "music", "testpcm.json"] }) },
    );

    expect(manifestResponse.status).toBe(200);
    expect(manifestResponse.headers.get("Content-Type")).toContain("application/json");
    const manifest = await manifestResponse.json();
    expect(manifest).toEqual(
      expect.objectContaining({
        kind: "music",
        sampleRate: 44_100,
        channelCount: 2,
        stems: [
          expect.objectContaining({ channel: 1, bitsPerSample: 16 }),
          expect.objectContaining({ channel: 2, bitsPerSample: 16 }),
        ],
      }),
    );
    expect(manifest.durationFrames).toBeLessThan(60);
    expect(manifest.stems[0].path).toBe("/api/audio/pcm/music/testpcm/ch1.pcm");

    const rawResponse = await GET(
      new Request("https://example.com/api/audio/pcm/music/testpcm/ch1.pcm"),
      { params: Promise.resolve({ path: ["pcm", "music", "testpcm", "ch1.pcm"] }) },
    );
    expect(rawResponse.status).toBe(200);
    expect(rawResponse.headers.get("Content-Type")).toBe("application/octet-stream");
    expect((await rawResponse.arrayBuffer()).byteLength).toBeGreaterThan(44);
  });

  it("serves direct PCM SFX manifests and raw clip data", async () => {
    const manifestResponse = await GET(
      new Request("https://example.com/api/audio/pcm/sfx/testsynth.json"),
      { params: Promise.resolve({ path: ["pcm", "sfx", "testsynth.json"] }) },
    );

    expect(manifestResponse.status).toBe(200);
    const manifest = await manifestResponse.json();
    expect(manifest).toEqual(
      expect.objectContaining({
        kind: "sfx",
        path: "/api/audio/pcm/sfx/testsynth.pcm",
        sampleRate: 44_100,
        bitsPerSample: 16,
        ownedChannels: [8],
      }),
    );

    const rawResponse = await GET(
      new Request("https://example.com/api/audio/pcm/sfx/testsynth.pcm"),
      { params: Promise.resolve({ path: ["pcm", "sfx", "testsynth.pcm"] }) },
    );
    expect(rawResponse.status).toBe(200);
    expect((await rawResponse.arrayBuffer()).byteLength).toBeGreaterThan(0);
  });

  it("serves direct PCM cry manifests for legacy cry slugs", async () => {
    const response = await GET(
      new Request("https://example.com/api/audio/pcm/cries/nidoran_m.json"),
      { params: Promise.resolve({ path: ["pcm", "cries", "nidoran_m.json"] }) },
    );

    expect(response.status).toBe(200);
    const manifest = await response.json();
    expect(manifest).toEqual(
      expect.objectContaining({
        kind: "cry",
        path: "/api/audio/pcm/cries/nidoran_m.pcm",
        sampleRate: 44_100,
        bitsPerSample: 16,
        ownedChannels: [5],
        priorityClass: "cry",
      }),
    );
  });

  it("returns 404 for missing bundled audio assets", async () => {
    const response = await GET(
      new Request("https://example.com/api/audio/not-real.mp3"),
      { params: Promise.resolve({ path: ["not-real.mp3"] }) },
    );

    expect(response.status).toBe(404);
  });

  it("does not synthesize music mp3 assets when the bundled file is missing", async () => {
    const response = await GET(
      new Request("https://example.com/api/audio/missingbundledtrack.mp3"),
      { params: Promise.resolve({ path: ["missingbundledtrack.mp3"] }) },
    );

    expect(response.status).toBe(404);
  });

  it("rejects traversal attempts outside the bundled audio roots", async () => {
    const response = await GET(
      new Request("https://example.com/api/audio/../../secrets.wav"),
      { params: Promise.resolve({ path: ["..", "..", "secrets.wav"] }) },
    );

    expect(response.status).toBe(404);
  });
});
