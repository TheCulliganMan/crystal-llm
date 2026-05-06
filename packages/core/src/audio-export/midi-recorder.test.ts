import { MidiRecorder } from "@pokecrystal/core/audio-export/midi-recorder";

describe("MidiRecorder", () => {
  it("builds a valid SMF header and track", () => {
    const recorder = new MidiRecorder({ sampleRate: 44_100 });
    recorder.recordNote({ channel: 0, note: 60, velocity: 100, startSample: 0, durationSamples: 22050 });
    recorder.setLoopPoints({ startSample: 0, endSample: 22050 });
    const bytes = recorder.toBytes();

    const ascii = new TextDecoder().decode(bytes.slice(0, 4));
    expect(ascii).toBe("MThd");
    expect(bytes.length).toBeGreaterThan(32);
  });
});
