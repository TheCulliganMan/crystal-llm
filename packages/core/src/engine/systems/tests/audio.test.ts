import {
  AudioEngine,
  type PcmAudioPlaybackBackend,
  type PcmBackendVoice,
} from "@pokecrystal/core/engine/systems/audio";
import { GB_FRAME_DURATION_MS } from "@pokecrystal/core/core/gb-timing";

class FakeAudio {
  public loop = false;
  public volume = 1;
  public muted = false;
  public paused = true;
  public currentTime = 0;
  public ended = false;
  private listeners = new Map<string, Array<() => void>>();
  public play = jest.fn(() => {
    this.paused = false;
    return Promise.resolve();
  });
  public pause = jest.fn(() => {
    this.paused = true;
  });
  public addEventListener = jest.fn((event: string, listener: () => void) => {
    const existing = this.listeners.get(event) ?? [];
    existing.push(listener);
    this.listeners.set(event, existing);
  });

  public emit(event: string): void {
    const listeners = this.listeners.get(event) ?? [];
    for (const listener of listeners) {
      listener();
    }
  }

  constructor(public readonly src: string = "") {}
}

class FakePcmBackend implements PcmAudioPlaybackBackend {
  public voices: PcmBackendVoice[] = [];
  public updates: Array<{ id: number; patch: Partial<Pick<PcmBackendVoice, "volume" | "muted" | "pan" | "playbackRate">> }> = [];
  public stopped: number[] = [];
  public resumed = 0;
  public disposed = 0;
  private readonly endedCallbacks = new Map<number, () => void>();

  playVoice(voice: PcmBackendVoice, onEnded?: () => void): void {
    this.voices.push(voice);
    if (onEnded) {
      this.endedCallbacks.set(voice.id, onEnded);
    }
  }

  stopVoice(id: number): void {
    this.stopped.push(id);
    this.endedCallbacks.delete(id);
  }

  updateVoice(id: number, patch: Partial<Pick<PcmBackendVoice, "volume" | "muted" | "pan" | "playbackRate">>): void {
    this.updates.push({ id, patch });
  }

  async resume(): Promise<void> {
    this.resumed += 1;
  }

  dispose(): void {
    this.disposed += 1;
    this.endedCallbacks.clear();
  }

  finish(id: number): void {
    this.endedCallbacks.get(id)?.();
  }
}

const pcmArrayBuffer = (samples: number[]): ArrayBuffer => {
  const buffer = new ArrayBuffer(samples.length * 2);
  const view = new DataView(buffer);
  samples.forEach((sample, index) => {
    view.setInt16(index * 2, sample, true);
  });
  return buffer;
};

const jsonResponse = (payload: unknown): Response =>
  ({
    ok: true,
    json: async () => payload,
  }) as unknown as Response;

const pcmResponse = (samples: number[]): Response =>
  ({
    ok: true,
    arrayBuffer: async () => pcmArrayBuffer(samples),
  }) as unknown as Response;

const flushPromises = async (): Promise<void> => {
  await Promise.resolve();
  await Promise.resolve();
  await new Promise((resolve) => setImmediate(resolve));
};

describe("AudioEngine direct PCM backend", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("plays PCM music stems through the injected backend", async () => {
    const backend = new FakePcmBackend();
    globalThis.fetch = jest.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("test.json")) {
        return jsonResponse({
          kind: "music",
          token: "MUSIC_TEST",
          sampleRate: 44_100,
          channelCount: 2,
          durationFrames: 8,
          loopStartSample: 0,
          loopEndSample: 2,
          stems: [
            {
              kind: "music",
              token: "MUSIC_TEST",
              channel: 1,
              path: "/api/audio/pcm/music/test/ch1.pcm",
              sampleRate: 44_100,
              channels: 2,
              bitsPerSample: 16,
              durationFrames: 8,
              loopStartSample: 0,
              loopEndSample: 2,
              ownedChannels: [1],
              priorityClass: "none",
            },
            {
              kind: "music",
              token: "MUSIC_TEST",
              channel: 2,
              path: "/api/audio/pcm/music/test/ch2.pcm",
              sampleRate: 44_100,
              channels: 2,
              bitsPerSample: 16,
              durationFrames: 8,
              loopStartSample: 0,
              loopEndSample: 2,
              ownedChannels: [2],
              priorityClass: "none",
            },
          ],
        });
      }
      return pcmResponse([100, 100, -100, -100]);
    }) as unknown as typeof globalThis.fetch;

    const engine = new AudioEngine({ playbackBackend: "direct-pcm", pcmBackend: backend });
    engine.loadMusic("MUSIC_TEST", "/api/audio/pcm/music/test.json");
    engine.playMusic("MUSIC_TEST", "map");
    await flushPromises();

    expect(backend.voices).toHaveLength(2);
    expect(backend.voices.map((voice) => [voice.kind, voice.loop, voice.loopStartSample, voice.loopEndSample])).toEqual([
      ["music", true, 0, 2],
      ["music", true, 0, 2],
    ]);
    expect(engine.getPlaybackSnapshot().activeChannels).toEqual([
      expect.objectContaining({ channel: 1, category: "music" }),
      expect.objectContaining({ channel: 2, category: "music" }),
    ]);
  });

  it("updates active PCM voice volume when the master volume changes", async () => {
    const backend = new FakePcmBackend();
    globalThis.fetch = jest.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("test.json")) {
        return jsonResponse({
          kind: "music",
          token: "MUSIC_TEST",
          sampleRate: 44_100,
          channelCount: 1,
          durationFrames: 8,
          loopStartSample: 0,
          loopEndSample: 2,
          stems: [
            {
              kind: "music",
              token: "MUSIC_TEST",
              channel: 1,
              path: "/api/audio/pcm/music/test/ch1.pcm",
              sampleRate: 44_100,
              channels: 2,
              bitsPerSample: 16,
              durationFrames: 8,
              loopStartSample: 0,
              loopEndSample: 2,
              ownedChannels: [1],
              priorityClass: "none",
            },
          ],
        });
      }
      return pcmResponse([100, 100, -100, -100]);
    }) as unknown as typeof globalThis.fetch;

    const engine = new AudioEngine({ playbackBackend: "direct-pcm", pcmBackend: backend });
    engine.loadMusic("MUSIC_TEST", "/api/audio/pcm/music/test.json");
    engine.playMusic("MUSIC_TEST", "map");
    await flushPromises();

    const voiceId = backend.voices[0].id;
    engine.setMasterVolume(0.25);

    expect(engine.masterVolume).toBe(0.25);
    expect(backend.updates).toContainEqual({
      id: voiceId,
      patch: expect.objectContaining({ volume: 0.25 }),
    });
  });

  it("plays PCM SFX, tracks waits, and releases on backend end", async () => {
    const backend = new FakePcmBackend();
    globalThis.fetch = jest.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("item.json")) {
        return jsonResponse({
          kind: "sfx",
          token: "SFX_ITEM",
          path: "/api/audio/pcm/sfx/item.pcm",
          sampleRate: 44_100,
          channels: 2,
          bitsPerSample: 16,
          durationFrames: 4,
          loopStartSample: null,
          loopEndSample: null,
          ownedChannels: [5],
          priorityClass: "none",
        });
      }
      return pcmResponse([200, 200, -200, -200]);
    }) as unknown as typeof globalThis.fetch;

    const engine = new AudioEngine({ playbackBackend: "direct-pcm", pcmBackend: backend });
    engine.loadSound("SFX_ITEM", "/api/audio/pcm/sfx/item.json");
    engine.playSound("SFX_ITEM", { panning: "left", pitch: 12 });
    await flushPromises();

    expect(backend.voices).toHaveLength(1);
    expect(backend.voices[0]).toEqual(expect.objectContaining({
      kind: "sfx",
      token: "SFX_ITEM",
      pan: -1,
      playbackRate: 2,
    }));
    expect(engine.isSoundPlaying()).toBe(true);

    backend.finish(backend.voices[0].id);

    expect(engine.isSoundPlaying()).toBe(false);
  });

  it("suppresses overlapping music channels while PCM SFX owns those channels", async () => {
    const backend = new FakePcmBackend();
    globalThis.fetch = jest.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("music.json")) {
        return jsonResponse({
          kind: "music",
          token: "MUSIC_TEST",
          sampleRate: 44_100,
          channelCount: 1,
          durationFrames: 8,
          loopStartSample: 0,
          loopEndSample: 2,
          stems: [
            {
              kind: "music",
              token: "MUSIC_TEST",
              channel: 1,
              path: "/api/audio/pcm/music/test/ch1.pcm",
              sampleRate: 44_100,
              channels: 2,
              bitsPerSample: 16,
              durationFrames: 8,
              loopStartSample: 0,
              loopEndSample: 2,
              ownedChannels: [1],
              priorityClass: "none",
            },
          ],
        });
      }
      if (url.endsWith("fanfare.json")) {
        return jsonResponse({
          kind: "sfx",
          token: "SFX_FANFARE",
          path: "/api/audio/pcm/sfx/fanfare.pcm",
          sampleRate: 44_100,
          channels: 2,
          bitsPerSample: 16,
          durationFrames: 4,
          loopStartSample: null,
          loopEndSample: null,
          ownedChannels: [5],
          priorityClass: "priority",
        });
      }
      return pcmResponse([100, 100, -100, -100]);
    }) as unknown as typeof globalThis.fetch;

    const engine = new AudioEngine({ playbackBackend: "direct-pcm", pcmBackend: backend });
    engine.loadMusic("MUSIC_TEST", "/api/audio/pcm/music/music.json");
    engine.loadSound("SFX_FANFARE", "/api/audio/pcm/sfx/fanfare.json");
    engine.playMusic("MUSIC_TEST", "map");
    await flushPromises();
    const musicVoiceId = backend.voices[0].id;

    engine.playSound("SFX_FANFARE");
    await flushPromises();

    expect(backend.updates).toContainEqual({
      id: musicVoiceId,
      patch: expect.objectContaining({ muted: true }),
    });

    backend.finish(backend.voices.at(-1)!.id);

    expect(backend.updates).toContainEqual({
      id: musicVoiceId,
      patch: expect.objectContaining({ muted: false }),
    });
  });
});

describe("AudioEngine unlock", () => {
  type TestWindow = { Audio?: typeof FakeAudio };
  const globalAny = globalThis as unknown as { window?: TestWindow };
  const hadWindow = "window" in globalAny;
  const originalWindow = globalAny.window;

  beforeEach(() => {
    globalAny.window = { Audio: FakeAudio };
  });

  afterEach(() => {
    if (!hadWindow) {
      delete globalAny.window;
      return;
    }
    globalAny.window = originalWindow;
  });

  it("does not restart music when already playing", () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_MOM", "mom.mp3");
    engine.playMusic("MUSIC_MOM", "map");
    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic).toBeTruthy();
    currentMusic.currentTime = 12;
    currentMusic.paused = false;

    const playMusicSpy = jest.spyOn(engine, "playMusic");
    playMusicSpy.mockClear();
    const playCalls = currentMusic.play.mock.calls.length;

    engine.unlock();

    expect(playMusicSpy).not.toHaveBeenCalled();
    expect((engine as unknown as { currentMusic: FakeAudio }).currentMusic).toBe(currentMusic);
    expect(currentMusic.currentTime).toBe(12);
    expect(currentMusic.play).toHaveBeenCalledTimes(playCalls);
  });

  it("retries play when music is paused without restarting", () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_MOM", "mom.mp3");
    engine.playMusic("MUSIC_MOM", "map");
    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    currentMusic.paused = true;

    const playMusicSpy = jest.spyOn(engine, "playMusic");
    playMusicSpy.mockClear();
    const playCalls = currentMusic.play.mock.calls.length;

    engine.unlock();

    expect(playMusicSpy).not.toHaveBeenCalled();
    expect(currentMusic.play).toHaveBeenCalledTimes(playCalls + 1);
  });

  it("does not resume cleared map music on unlock", () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_MOM", "mom.mp3");
    engine.playMusic("MUSIC_MOM", "map");
    engine.clearMapMusic();
    engine.stopMusic();

    const playMusicSpy = jest.spyOn(engine, "playMusic");
    playMusicSpy.mockClear();

    engine.unlock();

    expect(playMusicSpy).not.toHaveBeenCalled();
  });

  it("resumes from the tracked playback frame instead of restarting", () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_MOM", "mom.mp3");
    engine.playMusic("MUSIC_MOM", "map");
    engine.update();
    engine.update();
    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    currentMusic.paused = true;

    engine.unlock();

    expect(currentMusic.currentTime).toBeCloseTo((2 * GB_FRAME_DURATION_MS) / 1000, 2);
    expect(currentMusic.play).toHaveBeenCalled();
  });
});

describe("AudioEngine priority muting", () => {
  type TestWindow = { Audio?: typeof FakeAudio };
  const globalAny = globalThis as unknown as { window?: TestWindow };
  const hadWindow = "window" in globalAny;
  const originalWindow = globalAny.window;

  beforeEach(() => {
    globalAny.window = { Audio: FakeAudio };
  });

  afterEach(() => {
    if (!hadWindow) {
      delete globalAny.window;
      return;
    }
    globalAny.window = originalWindow;
  });

  it("mutes and restores map music for priority sfx", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.playMusic("MUSIC_TEST", "map");

    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic.muted).toBe(false);

    engine.playSound("SFX_FANFARE");
    await Promise.resolve();

    expect(currentMusic.muted).toBe(true);

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [sound] = activeSounds.get("SFX_FANFARE") ?? [];
    expect(sound).toBeTruthy();
    sound.emit("ended");

    expect(currentMusic.muted).toBe(false);
  });

  it("mutes and restores map music for cries", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.playMusic("MUSIC_TEST", "map");

    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic.muted).toBe(false);

    engine.playSound("CRY_PIKACHU");
    await Promise.resolve();

    expect(currentMusic.muted).toBe(true);

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [sound] = activeSounds.get("CRY_PIKACHU") ?? [];
    expect(sound).toBeTruthy();
    sound.emit("ended");

    expect(currentMusic.muted).toBe(false);
  });

  it("mutes and restores map music for congratulatory fanfares", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.playMusic("MUSIC_TEST", "map");

    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic.muted).toBe(false);

    engine.playSound("SFX_CAUGHT_MON");
    await Promise.resolve();

    expect(currentMusic.muted).toBe(true);

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [sound] = activeSounds.get("SFX_CAUGHT_MON") ?? [];
    expect(sound).toBeTruthy();
    sound.emit("ended");

    expect(currentMusic.muted).toBe(false);
  });

  it("mutes and restores map music for dex fanfares", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.playMusic("MUSIC_TEST", "map");

    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic.muted).toBe(false);

    engine.playSound("SFX_DEX_FANFARE_50_79");
    await Promise.resolve();

    expect(currentMusic.muted).toBe(true);

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [sound] = activeSounds.get("SFX_DEX_FANFARE_50_79") ?? [];
    expect(sound).toBeTruthy();
    sound.emit("ended");

    expect(currentMusic.muted).toBe(false);
  });

  it("does not mute map music for non-priority sfx", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.playMusic("MUSIC_TEST", "map");

    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic.muted).toBe(false);

    engine.playSound("SFX_ITEM");
    await Promise.resolve();

    expect(currentMusic.muted).toBe(false);
  });

  it("mutes custom loaded cries that bypass manifest-backed token lookup", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.loadSound("wooper_cry", "wooper.mp3");
    engine.playMusic("MUSIC_TEST", "map");

    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic.muted).toBe(false);

    engine.playSound("wooper_cry");
    await Promise.resolve();

    expect(currentMusic.muted).toBe(true);

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [sound] = activeSounds.get("wooper_cry") ?? [];
    expect(sound).toBeTruthy();
    sound.emit("ended");

    expect(currentMusic.muted).toBe(false);
  });

  it("replaces current SFX when higher-priority SFX starts", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.playMusic("MUSIC_TEST", "map");

    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic.muted).toBe(false);

    engine.playSound("SFX_FANFARE");
    await Promise.resolve();

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [fanfare] = activeSounds.get("SFX_FANFARE") ?? [];
    expect(fanfare).toBeTruthy();
    expect(currentMusic.muted).toBe(true);

    engine.playSound("SFX_ITEM");
    await Promise.resolve();

    const [item] = activeSounds.get("SFX_ITEM") ?? [];
    expect(item).toBeTruthy();
    expect(activeSounds.get("SFX_FANFARE")).toBeUndefined();
    expect(fanfare?.paused).toBe(true);
    expect(currentMusic.muted).toBe(false);
  });

  it("skips lower-priority SFX while higher-priority SFX is active", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.playMusic("MUSIC_TEST", "map");

    engine.playSound("SFX_ITEM");
    await Promise.resolve();

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [item] = activeSounds.get("SFX_ITEM") ?? [];
    expect(item).toBeTruthy();
    expect(engine.isSoundPlaying()).toBe(true);

    engine.playSound("SFX_FANFARE");
    await Promise.resolve();

    expect(activeSounds.get("SFX_FANFARE")).toBeUndefined();
    expect(activeSounds.get("SFX_ITEM")?.length).toBe(1);
    expect(item?.paused).toBe(false);
    expect(engine.isSoundPlaying()).toBe(true);
  });

  it("checks any active SFX when no token is provided", async () => {
    const engine = new AudioEngine();
    engine.playSound("SFX_ITEM");
    await Promise.resolve();

    expect(engine.isSoundPlaying()).toBe(true);
  });

  it("does not keep paused SFX active for global waits", async () => {
    const engine = new AudioEngine();
    engine.playSound("SFX_ITEM");
    await Promise.resolve();

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [item] = activeSounds.get("SFX_ITEM") ?? [];
    expect(item).toBeTruthy();

    item.pause();

    expect(engine.isSoundPlaying()).toBe(false);
    expect(activeSounds.get("SFX_ITEM")).toBeUndefined();
  });

  it("includes cry playback in global SFX waits", async () => {
    const engine = new AudioEngine();
    engine.playSound("CRY_PIKACHU");
    await Promise.resolve();

    expect(engine.isSoundPlaying()).toBe(true);
    expect(engine.isSoundPlaying("CRY_PIKACHU")).toBe(true);
  });

  it("resolves aliased and case-variant sound tokens in named SFX checks", async () => {
    const engine = new AudioEngine();
    engine.playSound("menu_cursor");
    await Promise.resolve();

    expect(engine.isSoundPlaying("SFX_MENU")).toBe(true);
    expect(engine.isSoundPlaying("menu_cursor")).toBe(true);
  });

  it("clears cries with SFX channels", async () => {
    const engine = new AudioEngine();
    engine.playSound("CRY_PIKACHU");
    engine.playSound("SFX_ITEM");
    await Promise.resolve();

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [cry] = activeSounds.get("CRY_PIKACHU") ?? [];
    const [sfx] = activeSounds.get("SFX_ITEM") ?? [];
    expect(cry).toBeTruthy();
    expect(sfx).toBeTruthy();

    engine.sfxChannelsOff();

    expect(sfx?.paused).toBe(true);
    expect(cry?.paused).toBe(true);
  });

  it("still allows SFX playback while a cry is active", async () => {
    const engine = new AudioEngine();
    engine.playSound("CRY_PIKACHU");
    await Promise.resolve();
    engine.playSound("SFX_ITEM");
    await Promise.resolve();

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    expect(activeSounds.get("CRY_PIKACHU")?.length).toBe(1);
    expect(activeSounds.get("SFX_ITEM")?.length).toBe(1);
    expect(engine.isSoundPlaying()).toBe(true);
    expect(engine.getActiveSoundIds()).toEqual(["SFX_ITEM"]);
  });

  it("exposes active channel ownership in playback snapshots", async () => {
    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TEST", "test.mp3");
    engine.playMusic("MUSIC_TEST", "map");
    engine.playSound("SFX_ITEM");
    await Promise.resolve();

    const snapshot = engine.getPlaybackSnapshot();
    expect(snapshot.musicToken).toBe("MUSIC_TEST");
    expect(snapshot.musicSource).toBe("test.mp3");
    expect(snapshot.activeChannels.some((entry) => entry.category === "music")).toBe(true);
    expect(snapshot.recentEvents).toEqual([
      expect.objectContaining({
        sequence: 1,
        kind: "music",
        token: "MUSIC_TEST",
        source: "test.mp3",
        role: "map",
        loop: true,
      }),
      expect.objectContaining({
        sequence: 2,
        kind: "sfx",
        token: "SFX_ITEM",
        source: "/api/audio/sfx/item.mp3",
        loop: false,
      }),
    ]);
  });
});

describe("AudioEngine manifest failures", () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
  });

  it("treats missing music manifests as silence instead of throwing", async () => {
    globalThis.fetch = jest.fn(async () => ({
      ok: false,
      status: 404,
    })) as unknown as typeof globalThis.fetch;

    const engine = new AudioEngine();
    engine.loadMusic("MUSIC_TITLE", "/api/audio/manifests/music/titlescreen.json");
    (
      engine as unknown as {
        pendingMusicRequestId: number;
        currentMusicName: string | null;
      }
    ).pendingMusicRequestId = 1;
    (
      engine as unknown as {
        pendingMusicRequestId: number;
        currentMusicName: string | null;
      }
    ).currentMusicName = "MUSIC_TITLE";

    await expect(
      (
        engine as unknown as {
          _playMusicManifest: (
            token: string,
            source: string,
            role: string,
            requestId: number,
          ) => Promise<void>;
          pendingMusicRequestId: number;
          currentMusicName: string | null;
          currentMusicRole: string;
        }
      )._playMusicManifest("MUSIC_TITLE", "/api/audio/manifests/music/titlescreen.json", "title", 1)
    ).resolves.toBeUndefined();

    expect(
      (engine as unknown as { currentMusicName: string | null }).currentMusicName
    ).toBeNull();
    expect(
      (engine as unknown as { currentMusicRole: string }).currentMusicRole
    ).toBe("general");
  });

  it("ignores missing sound manifests instead of throwing", async () => {
    globalThis.fetch = jest.fn(async () => ({
      ok: false,
      status: 404,
    })) as unknown as typeof globalThis.fetch;

    const engine = new AudioEngine();
    (
      engine as unknown as {
        pendingSoundManifestRequestId: number;
      }
    ).pendingSoundManifestRequestId = 7;

    await expect(
      (
        engine as unknown as {
          _playSoundManifest: (
            token: string,
            source: string,
            options: Record<string, unknown>,
            requestId: number,
          ) => Promise<void>;
          pendingSoundManifestRequestId: number;
        }
      )._playSoundManifest("SFX_ITEM", "/api/audio/manifests/sfx/item.json", {}, 7)
    ).resolves.toBeUndefined();
  });
});

describe("AudioEngine music aliases", () => {
  type TestWindow = { Audio?: typeof FakeAudio };
  const globalAny = globalThis as unknown as { window?: TestWindow };
  const hadWindow = "window" in globalAny;
  const originalWindow = globalAny.window;

  beforeEach(() => {
    globalAny.window = { Audio: FakeAudio };
  });

  afterEach(() => {
    if (!hadWindow) {
      delete globalAny.window;
      return;
    }
    globalAny.window = originalWindow;
  });

  it("maps gym leader battle music to the disassembly pointer asset", () => {
    const engine = new AudioEngine();
    engine.playMusic("MUSIC_KANTO_GYM_LEADER_BATTLE");
    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic).toBeTruthy();
    expect(currentMusic.src).toBe("/api/audio/kantogymbattle.mp3");
  });

  it("maps wild victory music to the disassembly pointer asset", () => {
    const engine = new AudioEngine();
    engine.playMusic("MUSIC_WILD_VICTORY");
    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic).toBeTruthy();
    expect(currentMusic.src).toBe("/api/audio/wildpokemonvictory.mp3");
  });

  it("maps crystal intro music to the baked intro asset", () => {
    const engine = new AudioEngine();
    engine.playMusic("MUSIC_CRYSTAL_OPENING");
    const currentMusic = (engine as unknown as { currentMusic: FakeAudio }).currentMusic;
    expect(currentMusic).toBeTruthy();
    expect(currentMusic.src).toBe("/api/audio/crystalopening.mp3");
  });
});

describe("AudioEngine intro SFX aliases", () => {
  type TestWindow = { Audio?: typeof FakeAudio };
  const globalAny = globalThis as unknown as { window?: TestWindow };
  const hadWindow = "window" in globalAny;
  const originalWindow = globalAny.window;

  beforeEach(() => {
    globalAny.window = { Audio: FakeAudio };
  });

  afterEach(() => {
    if (!hadWindow) {
      delete globalAny.window;
      return;
    }
    globalAny.window = originalWindow;
  });

  it.each([
    ["SFX_INTRO_UNOWN_1", "/api/audio/sfx/introunown1.mp3"],
    ["SFX_INTRO_UNOWN_2", "/api/audio/sfx/introunown2.mp3"],
    ["SFX_INTRO_UNOWN_3", "/api/audio/sfx/introunown3.mp3"],
    ["SFX_INTRO_PICHU", "/api/audio/sfx/intropichu.mp3"],
    ["SFX_INTRO_SUICUNE_2", "/api/audio/sfx/introsuicune2.mp3"],
    ["SFX_INTRO_SUICUNE_3", "/api/audio/sfx/introsuicune3.mp3"],
    ["SFX_INTRO_SUICUNE_4", "/api/audio/sfx/introsuicune4.mp3"],
    ["SFX_INTRO_WHOOSH", "/api/audio/sfx/introwhoosh.mp3"],
  ])("maps %s to %s", async (token, expected) => {
    const engine = new AudioEngine();
    engine.playSound(token);
    await Promise.resolve();

    const activeSounds = (engine as unknown as { activeSounds: Map<string, FakeAudio[]> }).activeSounds;
    const [sound] = activeSounds.get(token) ?? [];
    expect(sound).toBeTruthy();
    expect(sound?.src).toBe(expected);
  });
});
