import { BaseUI } from './base-ui';
import { Surface } from './surface';

export class CanvasUI extends BaseUI {
  private readonly canvas: HTMLCanvasElement;
  private readonly ctx: CanvasRenderingContext2D;

  constructor(
    canvas: HTMLCanvasElement,
    screenWidth: number = 160,
    screenHeight: number = 144,
    scale: number = 1,
  ) {
    super(screenWidth, screenHeight, scale);
    this.canvas = canvas;
    this.canvas.width = this.screenWidth * this.scale;
    this.canvas.height = this.screenHeight * this.scale;
    const context = this.canvas.getContext('2d');
    if (!context) {
      throw new Error('Could not get 2D rendering context from canvas');
    }
    this.ctx = context;
    this.ctx.imageSmoothingEnabled = false;
  }

  protected createScreenSurface(): Surface {
    return new Surface(this.screenWidth, this.screenHeight);
  }

  update() {
    this.ctx.putImageData(this.screen.getImageData(), 0, 0);
    if (this.scale > 1) {
      this.ctx.drawImage(
        this.screen.canvas as unknown as CanvasImageSource,
        0,
        0,
        this.screenWidth,
        this.screenHeight,
        0,
        0,
        this.screenWidth * this.scale,
        this.screenHeight * this.scale
      );
    }
  }
}
