export type ImageDataLike = ImageData;

const hasImageData = typeof ImageData !== "undefined";

export const createImageData = (width: number, height: number): ImageDataLike => {
  if (hasImageData) {
    return new ImageData(width, height);
  }
  return { data: new Uint8ClampedArray(width * height * 4), width, height } as ImageDataLike;
};

const clampByte = (value: number): number => Math.max(0, Math.min(255, value));

const parseRgba = (value: string): [number, number, number, number] => {
  const match = value.match(/rgba?\(([^)]+)\)/i);
  if (!match) {
    return [0, 0, 0, 255];
  }
  const parts = match[1].split(",").map((entry) => entry.trim());
  const r = clampByte(Number(parts[0] ?? 0));
  const g = clampByte(Number(parts[1] ?? 0));
  const b = clampByte(Number(parts[2] ?? 0));
  const alphaRaw = parts[3] ?? "1";
  const alphaValue = Number(alphaRaw);
  const alpha = Number.isFinite(alphaValue)
    ? alphaValue <= 1
      ? alphaValue * 255
      : alphaValue
    : 255;
  return [r, g, b, clampByte(alpha)];
};

type HeadlessDrawingState = {
  fillStyle: string;
  strokeStyle: string;
  lineWidth: number;
  globalAlpha: number;
  globalCompositeOperation: string;
  imageSmoothingEnabled: boolean;
};

export class HeadlessContext2D {
  private readonly width: number;
  private readonly height: number;
  private readonly data: Uint8ClampedArray;
  private readonly stateStack: HeadlessDrawingState[] = [];
  public readonly canvas: { width: number; height: number };

  public fillStyle = "rgba(0,0,0,1)";
  public strokeStyle = "rgba(0,0,0,1)";
  public lineWidth = 1;
  public globalAlpha = 1;
  public globalCompositeOperation = "source-over";
  public imageSmoothingEnabled = false;

  constructor(
    width: number,
    height: number,
    data?: Uint8ClampedArray,
    canvas?: { width: number; height: number }
  ) {
    this.width = width;
    this.height = height;
    this.data = data ?? new Uint8ClampedArray(width * height * 4);
    this.canvas = canvas ?? { width, height };
  }

  private captureState(): HeadlessDrawingState {
    return {
      fillStyle: this.fillStyle,
      strokeStyle: this.strokeStyle,
      lineWidth: this.lineWidth,
      globalAlpha: this.globalAlpha,
      globalCompositeOperation: this.globalCompositeOperation,
      imageSmoothingEnabled: this.imageSmoothingEnabled,
    };
  }

  private applyState(state: HeadlessDrawingState): void {
    this.fillStyle = state.fillStyle;
    this.strokeStyle = state.strokeStyle;
    this.lineWidth = state.lineWidth;
    this.globalAlpha = state.globalAlpha;
    this.globalCompositeOperation = state.globalCompositeOperation;
    this.imageSmoothingEnabled = state.imageSmoothingEnabled;
  }

  getImageData(x: number, y: number, width: number, height: number): ImageDataLike {
    const image = createImageData(width, height);
    const output = image.data;
    for (let row = 0; row < height; row += 1) {
      const srcY = y + row;
      for (let col = 0; col < width; col += 1) {
        const srcX = x + col;
        const srcIndex = (srcY * this.width + srcX) * 4;
        const dstIndex = (row * width + col) * 4;
        output[dstIndex] = this.data[srcIndex] ?? 0;
        output[dstIndex + 1] = this.data[srcIndex + 1] ?? 0;
        output[dstIndex + 2] = this.data[srcIndex + 2] ?? 0;
        output[dstIndex + 3] = this.data[srcIndex + 3] ?? 0;
      }
    }
    return image;
  }

  putImageData(image: ImageDataLike, x: number, y: number): void {
    const width = image.width;
    const height = image.height;
    const input = image.data;
    for (let row = 0; row < height; row += 1) {
      const destY = y + row;
      for (let col = 0; col < width; col += 1) {
        const destX = x + col;
        const dstIndex = (destY * this.width + destX) * 4;
        const srcIndex = (row * width + col) * 4;
        this.data[dstIndex] = input[srcIndex] ?? 0;
        this.data[dstIndex + 1] = input[srcIndex + 1] ?? 0;
        this.data[dstIndex + 2] = input[srcIndex + 2] ?? 0;
        this.data[dstIndex + 3] = input[srcIndex + 3] ?? 0;
      }
    }
  }

  createImageData(width: number, height: number): ImageDataLike {
    return createImageData(width, height);
  }

  fillRect(x: number, y: number, width: number, height: number): void {
    const [r, g, b, a] = parseRgba(this.fillStyle);
    for (let row = 0; row < height; row += 1) {
      const destY = y + row;
      for (let col = 0; col < width; col += 1) {
        const destX = x + col;
        const index = (destY * this.width + destX) * 4;
        this.data[index] = r;
        this.data[index + 1] = g;
        this.data[index + 2] = b;
        this.data[index + 3] = a;
      }
    }
  }

  clearRect(x: number, y: number, width: number, height: number): void {
    const startX = Math.max(0, Math.floor(x));
    const startY = Math.max(0, Math.floor(y));
    const endX = Math.min(this.width, Math.ceil(x + width));
    const endY = Math.min(this.height, Math.ceil(y + height));
    for (let row = startY; row < endY; row += 1) {
      for (let col = startX; col < endX; col += 1) {
        const index = (row * this.width + col) * 4;
        this.data[index] = 0;
        this.data[index + 1] = 0;
        this.data[index + 2] = 0;
        this.data[index + 3] = 0;
      }
    }
  }

  strokeRect(x: number, y: number, width: number, height: number): void {
    const [r, g, b, a] = parseRgba(this.strokeStyle);
    const drawPixel = (px: number, py: number) => {
      if (px < 0 || py < 0 || px >= this.width || py >= this.height) {
        return;
      }
      const index = (py * this.width + px) * 4;
      this.data[index] = r;
      this.data[index + 1] = g;
      this.data[index + 2] = b;
      this.data[index + 3] = a;
    };
    const maxX = Math.floor(x + width - 1);
    const maxY = Math.floor(y + height - 1);
    for (let px = Math.floor(x); px <= maxX; px += 1) {
      drawPixel(px, Math.floor(y));
      drawPixel(px, maxY);
    }
    for (let py = Math.floor(y); py <= maxY; py += 1) {
      drawPixel(Math.floor(x), py);
      drawPixel(maxX, py);
    }
  }

  drawImage(
    source: CanvasImageSource | { getContext?: (type: string) => HeadlessContext2D | null; width?: number; height?: number },
    ...args: number[]
  ): void {
    const srcCtx = canReadHeadlessContext(source) ? source.getContext("2d") : null;
    if (!srcCtx || !(srcCtx instanceof HeadlessContext2D)) {
      return;
    }
    let sx = 0;
    let sy = 0;
    let sw = srcCtx.width;
    let sh = srcCtx.height;
    let dx = 0;
    let dy = 0;
    let dw = sw;
    let dh = sh;

    if (args.length === 2) {
      [dx, dy] = args;
    } else if (args.length === 4) {
      [dx, dy, dw, dh] = args;
    } else if (args.length === 8) {
      [sx, sy, sw, sh, dx, dy, dw, dh] = args;
    } else {
      return;
    }

    const alphaScale = Math.max(0, Math.min(1, this.globalAlpha));
    for (let y = 0; y < dh; y += 1) {
      const srcY = Math.floor(sy + (y / dh) * sh);
      for (let x = 0; x < dw; x += 1) {
        const srcX = Math.floor(sx + (x / dw) * sw);
        const srcIndex = (srcY * srcCtx.width + srcX) * 4;
        const dstIndex = ((dy + y) * this.width + (dx + x)) * 4;
        const srcAlpha = (srcCtx.data[srcIndex + 3] ?? 0) / 255;
        const blendedAlpha = srcAlpha * alphaScale;
        if (blendedAlpha <= 0) {
          continue;
        }
        const invAlpha = 1 - blendedAlpha;
        const srcR = srcCtx.data[srcIndex] ?? 0;
        const srcG = srcCtx.data[srcIndex + 1] ?? 0;
        const srcB = srcCtx.data[srcIndex + 2] ?? 0;
        const dstR = this.data[dstIndex] ?? 0;
        const dstG = this.data[dstIndex + 1] ?? 0;
        const dstB = this.data[dstIndex + 2] ?? 0;
        const dstA = (this.data[dstIndex + 3] ?? 0) / 255;
        const outA = blendedAlpha + dstA * invAlpha;
        const outR = Math.round(srcR * blendedAlpha + dstR * invAlpha);
        const outG = Math.round(srcG * blendedAlpha + dstG * invAlpha);
        const outB = Math.round(srcB * blendedAlpha + dstB * invAlpha);
        this.data[dstIndex] = outR;
        this.data[dstIndex + 1] = outG;
        this.data[dstIndex + 2] = outB;
        this.data[dstIndex + 3] = Math.round(outA * 255);
      }
    }
  }

  beginPath(): void {
    return;
  }

  moveTo(_x: number, _y: number): void {
    return;
  }

  lineTo(_x: number, _y: number): void {
    return;
  }

  stroke(): void {
    return;
  }

  arc(_x: number, _y: number, _radius: number, _start: number, _end: number): void {
    return;
  }

  ellipse(
    _x: number,
    _y: number,
    _radiusX: number,
    _radiusY: number,
    _rotation: number,
    _start: number,
    _end: number
  ): void {
    return;
  }

  fill(): void {
    return;
  }

  save(): void {
    this.stateStack.push(this.captureState());
  }

  restore(): void {
    const state = this.stateStack.pop();
    if (state) {
      this.applyState(state);
    }
  }

  setTransform(
    _a: number,
    _b: number,
    _c: number,
    _d: number,
    _e: number,
    _f: number
  ): void {
    return;
  }

  translate(_x: number, _y: number): void {
    return;
  }

  scale(_x: number, _y: number): void {
    return;
  }
}

const canReadHeadlessContext = (
  source: CanvasImageSource | { getContext?: (type: string) => HeadlessContext2D | null }
): source is { getContext: (type: string) => HeadlessContext2D | null } => {
  const candidate = source as { getContext?: (type: string) => HeadlessContext2D | null } | null;
  return typeof candidate === "object" && candidate !== null && typeof candidate.getContext === "function";
};

export class HeadlessCanvas {
  public width: number;
  public height: number;
  private readonly context: HeadlessContext2D;

  constructor(width: number, height: number, data?: Uint8ClampedArray) {
    this.width = width;
    this.height = height;
    this.context = new HeadlessContext2D(width, height, data, this);
  }

  getContext(type: "2d"): HeadlessContext2D | null {
    if (type !== "2d") {
      return null;
    }
    return this.context;
  }
}
