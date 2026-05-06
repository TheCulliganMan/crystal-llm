import { AudioEngine } from "@/engine/systems/audio";

type MockAudioInstance = {
  src: string;
  loop: boolean;
  volume: number;
  muted: boolean;
  paused: boolean;
  ended: boolean;
  currentTime: number;
  play: jest.Mock<Promise<void>, []>;
  pause: jest.Mock<void, []>;
  addEventListener: jest.Mock<void, [string, () => void, { once: true }]>;
};

type PlayImpl = () => Promise<void>;

const audioInstances: MockAudioInstance[] = [];
const playQueue: PlayImpl[] = [];
let mockAudioContext:
  | {
      state: "running" | "suspended";
      resume: jest.Mock<Promise<void>, []>;
      createStereoPanner: jest.Mock;
      createGain: jest.Mock;
      createMediaElementSource: jest.Mock;
      destination: object;
    }
  | null = null;

class MockAudio {
  public src: string;
  public loop = false;
  public volume = 1;
  public muted = false;
  public paused = false;
  public ended = false;
  public currentTime = 0;
  public play: jest.Mock<Promise<void>, []>;
  public pause: jest.Mock<void, []>;
  public addEventListener: jest.Mock<void, [string, () => void, { once: true }]>;

  constructor(src: string) {
    this.src = src;
    const playImpl = playQueue.shift() ?? (() => Promise.resolve());
    this.play = jest.fn(() => playImpl());
    this.pause = jest.fn(() => {
      this.paused = true;
    });
    this.addEventListener = jest.fn();
    audioInstances.push(this as unknown as MockAudioInstance);
  }
}

const flushPromises = async () => {
  await Promise.resolve();
};

const setWindowAudio = () => {
  (global as typeof globalThis & { window?: unknown }).window = {
    Audio: MockAudio,
    AudioContext: jest.fn(() => mockAudioContext),
  };
};

describe("AudioEngine", () => {
  const originalWindow = global.window;

  beforeEach(() => {
    audioInstances.length = 0;
    playQueue.length = 0;
    mockAudioContext = {
      state: "suspended",
      resume: jest.fn(async () => {
        if (mockAudioContext) {
          mockAudioContext.state = "running";
        }
      }),
      createStereoPanner: jest.fn(() => ({ pan: { value: 0 }, connect: jest.fn(), disconnect: jest.fn() })),
      createGain: jest.fn(() => ({ connect: jest.fn(), disconnect: jest.fn() })),
      createMediaElementSource: jest.fn(() => ({ connect: jest.fn(), disconnect: jest.fn() })),
      destination: {},
    };
    setWindowAudio();
  });

  afterEach(() => {
    (global as typeof globalThis & { window?: unknown }).window = originalWindow;
  });

  it("plays sounds immediately when allowed", async () => {
    const engine = new AudioEngine();
    engine.loadSound("SFX_MENU", "/audio/sfx/menu.mp3");
    engine.playSound("SFX_MENU");
    await flushPromises();

    expect(audioInstances).toHaveLength(1);
    expect(audioInstances[0].play).toHaveBeenCalledTimes(1);
    expect(engine.isSoundPlaying("SFX_MENU")).toBe(true);
    expect(engine.getActiveSoundIds()).toContain("SFX_MENU");
  });

  it("retries blocked sounds after unlock", async () => {
    playQueue.push(() => Promise.reject({ name: "NotAllowedError" }));
    playQueue.push(() => Promise.resolve());

    const engine = new AudioEngine();
    engine.loadSound("SFX_MENU", "/audio/sfx/menu.mp3");
    engine.playSound("SFX_MENU");
    await flushPromises();

    expect(audioInstances).toHaveLength(1);
    expect(audioInstances[0].play).toHaveBeenCalledTimes(1);
    expect(engine.isSoundPlaying("SFX_MENU")).toBe(false);

    engine.unlock();
    await flushPromises();

    expect(audioInstances).toHaveLength(2);
    expect(audioInstances[1].play).toHaveBeenCalledTimes(1);
    expect(engine.isSoundPlaying("SFX_MENU")).toBe(true);
  });

  it("resumes a suspended audio context during unlock before retrying playback", async () => {
    playQueue.push(() => Promise.reject({ name: "NotAllowedError" }));
    playQueue.push(() => Promise.resolve());

    const engine = new AudioEngine();
    engine.loadSound("SFX_MENU", "/audio/sfx/menu.mp3");
    engine.playSound("SFX_MENU", { panning: "left" });
    await flushPromises();

    expect(mockAudioContext?.resume).not.toHaveBeenCalled();

    engine.unlock();
    await flushPromises();

    expect(mockAudioContext?.resume).toHaveBeenCalledTimes(1);
    expect(engine.isSoundPlaying("SFX_MENU")).toBe(true);
  });

  it("resumes a suspended audio context before replaying paused music", async () => {
    const engine = new AudioEngine();
    engine.loadSound("SFX_MENU", "/audio/sfx/menu.mp3");
    engine.loadMusic("MUSIC_TEST", "/audio/music/test.mp3");
    engine.playSound("SFX_MENU", { panning: "left" });
    await flushPromises();
    engine.playMusic("MUSIC_TEST");
    await flushPromises();

    audioInstances[1].paused = true;

    engine.unlock();
    await flushPromises();

    expect(mockAudioContext?.resume).toHaveBeenCalledTimes(1);
    expect(audioInstances[1].play).toHaveBeenCalledTimes(2);
  });
});
