import { isDebugEnabled } from "@pokecrystal/core/core/debug-flags";
import { gameEngine } from "@pokecrystal/core/ui/game-engine";

export interface FrameMetric {
  label: string;
  durationMs: number;
}

type Surface = InstanceType<typeof gameEngine.Surface>;
const getTimestampMs = (): number =>
  typeof performance !== "undefined" && typeof performance.now === "function"
    ? performance.now()
    : Date.now();

export const isFlarePlotEnabled = (): boolean => isDebugEnabled("flare_plot");

export const beginFlarePlotFrame = (): number | null => {
  return isFlarePlotEnabled() ? getTimestampMs() : null;
};

export const finishFlarePlotFrame = (
  startMs: number | null,
  label: string,
  target: Surface | CanvasRenderingContext2D,
  x: number,
  y: number,
  width: number,
  height: number
): void => {
  if (startMs === null) {
    return;
  }
  globalFlarePlot.recordFrame(label, getTimestampMs() - startMs);
  globalFlarePlot.render(target, x, y, width, height);
};

export class FlarePlotRenderer {
  private metrics: Array<FrameMetric | undefined>;
  private metricCount = 0;
  private nextMetricIndex = 0;
  private maxFrames: number;
  private thresholdMs: number;

  constructor(maxFrames: number = 60, thresholdMs: number = 1000 / 60) {
    this.maxFrames = Math.max(1, Math.trunc(maxFrames));
    this.thresholdMs = thresholdMs;
    this.metrics = new Array(this.maxFrames);
  }

  recordFrame(label: string, durationMs: number): void {
    this.metrics[this.nextMetricIndex] = { label, durationMs };
    this.nextMetricIndex = (this.nextMetricIndex + 1) % this.maxFrames;
    if (this.metricCount < this.maxFrames) {
      this.metricCount += 1;
    }
  }

  getMetrics(): FrameMetric[] {
    const ordered: FrameMetric[] = [];
    this.forEachMetric((metric) => {
      ordered.push({ ...metric });
    });
    return ordered;
  }

  render(target: Surface | CanvasRenderingContext2D, x: number, y: number, width: number, height: number): void {
    if (!isFlarePlotEnabled()) {
      return;
    }

    if (this.metricCount === 0) {
      return;
    }

    const isSurface = "getContext" in target && typeof target.getContext === "function";
    const ctx = isSurface ? (target as Surface).getContext() : (target as CanvasRenderingContext2D);
    if (!ctx) return;

    ctx.save();

    // Basic background
    ctx.fillStyle = "rgba(0, 0, 0, 0.588)"; // 150/255
    ctx.fillRect(x, y, width, height);

    // Find max duration for scaling
    let maxDuration = this.thresholdMs * 2;
    this.forEachMetric((metric) => {
      if (metric.durationMs > maxDuration) {
        maxDuration = metric.durationMs;
      }
    });
    const barWidth = width / this.maxFrames;

    // Draw threshold line
    const thresholdY = y + height - (this.thresholdMs / maxDuration) * height;
    ctx.strokeStyle = "rgba(255, 255, 0, 1)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(x, thresholdY);
    ctx.lineTo(x + width, thresholdY);
    ctx.stroke();

    // Draw bars
    this.forEachMetric((metric, index) => {
      const barHeight = (metric.durationMs / maxDuration) * height;
      const barX = x + index * barWidth;
      const barY = y + height - barHeight;

      let color = "rgba(0, 255, 0, 1)"; // Normal green
      if (metric.durationMs > this.thresholdMs) {
        color = "rgba(255, 0, 0, 1)"; // Flare red
      }

      ctx.fillStyle = color;
      ctx.fillRect(barX, barY, Math.max(1, barWidth - 1), barHeight);
    });

    ctx.restore();
  }

  private forEachMetric(visitor: (metric: FrameMetric, index: number) => void): void {
    const startIndex = this.metricCount === this.maxFrames ? this.nextMetricIndex : 0;
    for (let index = 0; index < this.metricCount; index += 1) {
      const metric = this.metrics[(startIndex + index) % this.maxFrames];
      if (metric) {
        visitor(metric, index);
      }
    }
  }
}

export const globalFlarePlot = new FlarePlotRenderer();
