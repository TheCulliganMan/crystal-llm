import { BaseUI } from "./base-ui";
import { Surface } from "./surface";

export class DomCanvasUI extends BaseUI {
  private canvas: HTMLCanvasElement | null = null;
  private ctx: CanvasRenderingContext2D | null = null;

  constructor(
    screenWidth?: number | null,
    screenHeight?: number | null,
    scale: number = 1,
    _useSuperGameBoyTiles?: boolean | null,
    canvas?: HTMLCanvasElement | null
  ) {
    super(screenWidth ?? 160, screenHeight ?? 144, scale);
    if (canvas) {
      this.attachCanvas(canvas);
    } else if (typeof document !== "undefined") {
      const created = document.createElement("canvas");
      document.body.appendChild(created);
      this.attachCanvas(created);
    }
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  update(): void {
    if (!this.canvas || !this.ctx) {
      return;
    }
    this.ctx.drawImage(
      this.screen.canvas as HTMLCanvasElement,
      0,
      0,
      this.screenWidth,
      this.screenHeight,
      0,
      0,
      this.screenWidth * this.scale,
      this.screenHeight * this.scale
    );
    this.flush_window_stack();
  }

  private attachCanvas(canvas: HTMLCanvasElement): void {
    this.canvas = canvas;
    this.canvas.width = this.screenWidth * this.scale;
    this.canvas.height = this.screenHeight * this.scale;
    let context = this.canvas.getContext("2d", { willReadFrequently: true });
    if (!context) {
      context = this.canvas.getContext("2d");
    }
    if (!context) {
      throw new Error("Could not get 2D rendering context from canvas");
    }
    context.imageSmoothingEnabled = false;
    this.canvas.style.imageRendering = "pixelated";
    this.ctx = context;
  }
}
