import { z } from "zod";

export type RGBAColor = [number, number, number, number];

export const BufferFormatSchema = z.enum([
  "ARGB",
  "BGR",
  "BGRA",
  "P",
  "RGB",
  "RGBA",
  "RGBX",
]);
export type BufferFormat = z.infer<typeof BufferFormatSchema>;

export const ToBytesFormatSchema = z.enum([
  "ARGB",
  "ARGB_PREMULT",
  "BGR",
  "BGRA",
  "P",
  "RGB",
  "RGBA",
  "RGBA_PREMULT",
  "RGBX",
]);
export type ToBytesFormat = z.infer<typeof ToBytesFormatSchema>;

export interface BackendEvent {
  type: number;
  key?: number | null;
  unicode?: string | null;
}

export interface BackendAdapter {
  name: string;
  close(): void;
  createSurface(width: number, height: number, options?: { indexed?: boolean }): BackendSurface;
  surfaceFromBytes(
    payload: Uint8Array,
    width: number,
    height: number,
    format: BufferFormat
  ): BackendSurface;
  surfaceToBytes(surface: BackendSurface, format: ToBytesFormat): Uint8Array;
  loadPng(path: string): Promise<BackendSurface>;
  savePng(surface: BackendSurface, path: string): Promise<void>;
  blit(
    dest: BackendSurface,
    src: BackendSurface,
    rect: [number, number, number, number] | null
  ): void;
  fill(surface: BackendSurface, color: RGBAColor): void;
  scale(surface: BackendSurface, size: [number, number]): BackendSurface;
  setColorkey(surface: BackendSurface, color: RGBAColor | null): void;
  setPalette(surface: BackendSurface, palette: ReadonlyArray<ReadonlyArray<number>>): void;
  getPalette(surface: BackendSurface): RGBAColor[];
  getPixel(surface: BackendSurface, x: number, y: number): RGBAColor;
  setPixel(surface: BackendSurface, x: number, y: number, color: RGBAColor): void;
  getSize(surface: BackendSurface): [number, number];
  createWindow(
    width: number,
    height: number,
    options?: { scale?: number; headless?: boolean }
  ): BackendWindow;
  present(window: BackendWindow, surface: BackendSurface): void;
  pollEvents(window?: BackendWindow | null): BackendEvent[];
  ticks(): number;
}

export class BackendSurface {
  private readonly adapter: BackendAdapter;
  readonly raw: unknown;

  constructor(adapter: BackendAdapter, raw: unknown) {
    this.adapter = adapter;
    this.raw = raw;
  }

  getSize(): [number, number] {
    return this.adapter.getSize(this);
  }

  blit(src: BackendSurface, rect: [number, number, number, number] | null = null): void {
    this.adapter.blit(this, src, rect);
  }

  fill(color: RGBAColor): void {
    this.adapter.fill(this, color);
  }

  scale(size: [number, number]): BackendSurface {
    return this.adapter.scale(this, size);
  }

  setColorkey(color: RGBAColor | null): void {
    this.adapter.setColorkey(this, color);
  }

  setPalette(palette: ReadonlyArray<ReadonlyArray<number>>): void {
    this.adapter.setPalette(this, palette);
  }

  getPalette(): RGBAColor[] {
    return this.adapter.getPalette(this);
  }

  getPixel(x: number, y: number): RGBAColor {
    return this.adapter.getPixel(this, x, y);
  }

  setPixel(x: number, y: number, color: RGBAColor): void {
    this.adapter.setPixel(this, x, y, color);
  }

  toBytes(format: ToBytesFormat): Uint8Array {
    return this.adapter.surfaceToBytes(this, format);
  }
}

export class BackendWindow {
  private readonly adapter: BackendAdapter;
  readonly raw: unknown;

  constructor(adapter: BackendAdapter, raw: unknown) {
    this.adapter = adapter;
    this.raw = raw;
  }

  present(surface: BackendSurface): void {
    this.adapter.present(this, surface);
  }

  pollEvents(): BackendEvent[] {
    return this.adapter.pollEvents(this);
  }
}
