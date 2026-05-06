import { FrameMetrics, GameBenchmark } from "@/app/game-benchmark";

describe("GameBenchmark", () => {
  it("tracks recent frames up to the configured sample cap", () => {
    const benchmark = new GameBenchmark({ enabled: true, maxFrames: 2 }, () => 0);

    benchmark.beginFrame(1, 0);
    benchmark.recordPhase("handleInput", 2);
    benchmark.recordPhase("update", 3);
    benchmark.recordPhase("draw", 1);
    benchmark.endFrame(6);

    benchmark.beginFrame(2, 10);
    benchmark.recordPhase("handleInput", 1);
    benchmark.recordPhase("update", 2);
    benchmark.recordPhase("draw", 2);
    benchmark.endFrame(5);

    benchmark.beginFrame(3, 20);
    benchmark.recordPhase("handleInput", 4);
    benchmark.recordPhase("update", 4);
    benchmark.recordPhase("draw", 2);
    benchmark.endFrame(11);

    const recent = benchmark.getRecentFrames();
    expect(recent).toHaveLength(2);
    expect(recent[0].frame).toBe(3);
    expect(recent[1].frame).toBe(2);
    expect(recent[0].phaseDurations.handleInput).toBe(4);
  });

  it("calls the onFrame callback with each sample and respects slow-frame filtering", () => {
    const samples: FrameMetrics[] = [];
    const benchmark = new GameBenchmark(
      { enabled: true, onFrame: (metrics) => samples.push(metrics), maxFrames: 10 },
      () => 0,
    );

    benchmark.beginFrame(1, 0);
    benchmark.recordPhase("handleInput", 1);
    benchmark.recordPhase("update", 5);
    benchmark.recordPhase("draw", 2);
    benchmark.endFrame(8);

    benchmark.beginFrame(2, 10);
    benchmark.recordPhase("handleInput", 3);
    benchmark.recordPhase("update", 3);
    benchmark.recordPhase("draw", 3);
    benchmark.endFrame(9);

    expect(samples).toHaveLength(2);
    expect(samples[1].frame).toBe(2);
    const slow = benchmark.getSlowFrames(8.5);
    expect(slow).toContainEqual(expect.objectContaining({ frame: 2 }));
  });
});
