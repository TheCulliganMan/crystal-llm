import { BackendAdapter, BackendSurface, BufferFormat, RGBAColor, ToBytesFormat } from "./api";

export class Rect {
  constructor(
    public readonly x: number,
    public readonly y: number,
    public readonly w: number,
    public readonly h: number
  ) {}

  toTuple(): [number, number, number, number] {
    return [this.x, this.y, this.w, this.h];
  }

  clip(width: number, height: number): Rect {
    const clippedX = Math.max(0, Math.min(this.x, width));
    const clippedY = Math.max(0, Math.min(this.y, height));
    const clippedW = Math.max(0, Math.min(this.x + this.w, width) - clippedX);
    const clippedH = Math.max(0, Math.min(this.y + this.h, height) - clippedY);
    return new Rect(clippedX, clippedY, clippedW, clippedH);
  }
}

function normalizeRect(
  rect: Rect | [number, number, number, number] | null
): [number, number, number, number] | null {
  if (!rect) {
    return null;
  }
  if (rect instanceof Rect) {
    return rect.toTuple();
  }
  const [x, y, w, h] = rect;
  return [Math.trunc(x), Math.trunc(y), Math.trunc(w), Math.trunc(h)];
}

function rgbaTuple(color: ReadonlyArray<number>): RGBAColor {
  const [r, g, b, a = 255] = color;
  return [Math.trunc(r), Math.trunc(g), Math.trunc(b), Math.trunc(a)];
}

export class SurfaceController {
  private readonly backend: BackendAdapter;

  constructor(backend: BackendAdapter) {
    this.backend = backend;
  }

  create(width: number, height: number, options?: { indexed?: boolean }): BackendSurface {
    return this.backend.createSurface(width, height, options);
  }

  fromBytes(
    payload: Uint8Array,
    width: number,
    height: number,
    options: { format: BufferFormat }
  ): BackendSurface {
    return this.backend.surfaceFromBytes(payload, width, height, options.format);
  }

  toBytes(surface: BackendSurface, options: { format: ToBytesFormat }): Uint8Array {
    return this.backend.surfaceToBytes(surface, options.format);
  }

  loadPng(path: string): Promise<BackendSurface> {
    return this.backend.loadPng(path);
  }

  savePng(surface: BackendSurface, path: string): Promise<void> {
    return this.backend.savePng(surface, path);
  }

  blit(
    dest: BackendSurface,
    src: BackendSurface,
    rect: Rect | [number, number, number, number] | null = null
  ): void {
    this.backend.blit(dest, src, normalizeRect(rect));
  }

  fill(surface: BackendSurface, color: ReadonlyArray<number>): void {
    this.backend.fill(surface, rgbaTuple(color));
  }

  scale(surface: BackendSurface, size: [number, number]): BackendSurface {
    return this.backend.scale(surface, size);
  }

  setColorkey(surface: BackendSurface, color: ReadonlyArray<number> | null): void {
    this.backend.setColorkey(surface, color ? rgbaTuple(color) : null);
  }

  setPalette(surface: BackendSurface, palette: ReadonlyArray<ReadonlyArray<number>>): void {
    this.backend.setPalette(surface, palette);
  }

  getPalette(surface: BackendSurface): RGBAColor[] {
    return this.backend.getPalette(surface);
  }

  getPixel(surface: BackendSurface, x: number, y: number): RGBAColor {
    return this.backend.getPixel(surface, x, y);
  }

  setPixel(surface: BackendSurface, x: number, y: number, color: ReadonlyArray<number>): void {
    this.backend.setPixel(surface, x, y, rgbaTuple(color));
  }

  size(surface: BackendSurface): [number, number] {
    return this.backend.getSize(surface);
  }
}
