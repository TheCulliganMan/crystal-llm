const DEFAULT_MAX_FRAMES = 120;

export type GamePhase = "handleInput" | "update" | "draw";

export interface FrameMetrics {
  frame: number;
  timestamp: number;
  totalDuration: number;
  phaseDurations: Record<GamePhase, number>;
  state?: string;
}

export type GameBenchmarkOptions = {
  enabled?: boolean;
  maxFrames?: number;
  onFrame?: (metrics: FrameMetrics) => void;
};

export class GameBenchmark {
  private frames: FrameMetrics[] = [];
  private currentPhases: Partial<Record<GamePhase, number>> = {};
  private currentFrameNumber = 0;
  private currentTimestamp = 0;
  private currentState: string | undefined;
  private readonly now: () => number;

  constructor(private readonly options: GameBenchmarkOptions = {}, nowFn?: () => number) {
    this.now =
      nowFn ??
      (() => {
        if (typeof performance !== "undefined" && typeof performance.now === "function") {
          return performance.now();
        }
        return Date.now();
      });
  }

  public beginFrame(frameNumber: number, timestamp?: number, state?: string): void {
    this.currentFrameNumber = frameNumber;
    this.currentTimestamp = timestamp ?? this.now();
    this.currentState = state;
    this.currentPhases = {};
  }

  public recordPhase(phase: GamePhase, durationMs: number): void {
    this.currentPhases[phase] = durationMs;
  }

  public endFrame(totalDurationMs?: number): void {
    if (this.currentFrameNumber === 0 && this.currentTimestamp === 0) {
      return;
    }
    const duration = totalDurationMs ?? this.now() - this.currentTimestamp;
    const metrics: FrameMetrics = {
      frame: this.currentFrameNumber,
      timestamp: this.currentTimestamp,
      totalDuration: duration,
      phaseDurations: {
        handleInput: this.currentPhases.handleInput ?? 0,
        update: this.currentPhases.update ?? 0,
        draw: this.currentPhases.draw ?? 0,
      },
      state: this.currentState,
    };
    this.frames.unshift(metrics);
    while (this.frames.length > this.maxFrames()) {
      this.frames.pop();
    }
    if (this.options.onFrame) {
      try {
        this.options.onFrame(metrics);
      } catch {
        // Guard the benchmark from breaking the game loop.
      }
    }
  }

  public getRecentFrames(): FrameMetrics[] {
    return [...this.frames];
  }

  public getSlowFrames(thresholdMs: number): FrameMetrics[] {
    return this.frames.filter((entry) => entry.totalDuration >= thresholdMs);
  }

  public clear(): void {
    this.frames.length = 0;
  }

  private maxFrames(): number {
    return Math.max(1, this.options.maxFrames ?? DEFAULT_MAX_FRAMES);
  }
}
