/* Canvas-backed backend that mirrors the shared surface contract. */

import {
  BackendAdapter,
  BackendEvent,
  BackendSurface,
  BackendWindow,
  BufferFormat,
  BufferFormatSchema,
  RGBAColor,
  ToBytesFormat,
  ToBytesFormatSchema,
} from "./api";

const KEYDOWN = 768;
const KEYUP = 769;

type SurfaceMode = "RGBA" | "P";

interface MemorySurface {
  mode: SurfaceMode;
  width: number;
  height: number;
  data: Uint8ClampedArray | Uint8Array;
  palette: RGBAColor[];
  paletteBytes?: Uint8Array;
  colorkey: RGBAColor | null;
  imageDataCache?: ImageData;
  indexedImageDataCache?: ImageData;
  indexedRgbaCache?: Uint8ClampedArray;
  indexedCacheDirty?: boolean;
}

interface CanvasWindow {
  canvas: HTMLCanvasElement | OffscreenCanvas;
  context: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;
  width: number;
  height: number;
  scale: number;
  headless: boolean;
  scratch?: HTMLCanvasElement | OffscreenCanvas;
  scratchContext?: CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D;
}

interface WebBackendOptions {
  headless?: boolean;
  inputEnabled?: boolean;
}

function ensureCanvas(
  width: number,
  height: number,
  headless: boolean
): HTMLCanvasElement | OffscreenCanvas {
  if (!headless && typeof document !== "undefined") {
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    canvas.style.imageRendering = "pixelated";
    return canvas;
  }
  if (typeof OffscreenCanvas !== "undefined") {
    return new OffscreenCanvas(width, height);
  }
  if (typeof document !== "undefined") {
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    return canvas;
  }
  throw new Error("No canvas implementation available for web backend.");
}

function getCanvasContext(
  canvas: HTMLCanvasElement | OffscreenCanvas
): CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D {
  const ctx = canvas.getContext("2d") as
    | CanvasRenderingContext2D
    | OffscreenCanvasRenderingContext2D
    | null;
  if (!ctx) {
    throw new Error("Unable to access 2D canvas context.");
  }
  return ctx;
}

function clampColor(color: ReadonlyArray<number>): RGBAColor {
  const [r, g, b, a = 255] = color;
  return [
    Number(r) & 0xff,
    Number(g) & 0xff,
    Number(b) & 0xff,
    Number(a) & 0xff,
  ];
}

function ensurePalette(entries: ReadonlyArray<ReadonlyArray<number>>): RGBAColor[] {
  const palette: RGBAColor[] = [];
  for (let i = 0; i < 256; i += 1) {
    palette.push([0, 0, 0, 255]);
  }
  entries.forEach((entry, index) => {
    if (index >= 256) {
      return;
    }
    palette[index] = clampColor(entry);
  });
  return palette;
}

function paletteBytesFromPalette(palette: ReadonlyArray<RGBAColor>): Uint8Array {
  const bytes = new Uint8Array(256 * 4);
  for (let i = 0; i < 256; i += 1) {
    const color = palette[i] ?? [0, 0, 0, 255];
    const base = i * 4;
    bytes[base] = color[0];
    bytes[base + 1] = color[1];
    bytes[base + 2] = color[2];
    bytes[base + 3] = color[3];
  }
  return bytes;
}

function createSurface(width: number, height: number, indexed: boolean): MemorySurface {
  if (indexed) {
    const palette = ensurePalette([]);
    return {
      mode: "P",
      width,
      height,
      data: new Uint8Array(width * height),
      palette,
      paletteBytes: paletteBytesFromPalette(palette),
      colorkey: null,
      indexedCacheDirty: true,
    };
  }
  return {
    mode: "RGBA",
    width,
    height,
    data: new Uint8ClampedArray(width * height * 4),
    palette: [],
    colorkey: null,
  };
}

function assertSurface(surface: BackendSurface): MemorySurface {
  return surface.raw as MemorySurface;
}

function paletteIndexForColor(palette: RGBAColor[], color: RGBAColor): number {
  const [r, g, b] = color;
  for (let i = 0; i < palette.length; i += 1) {
    const [pr, pg, pb] = palette[i];
    if (pr === r && pg === g && pb === b) {
      return i;
    }
  }
  return -1;
}

function getPixel(surface: MemorySurface, x: number, y: number): RGBAColor {
  if (surface.mode === "P") {
    const index = surface.data[y * surface.width + x] as number;
    const entry = surface.palette[index];
    if (!entry) {
      throw new Error("Indexed pixel references missing palette entry.");
    }
    return entry;
  }
  const base = (y * surface.width + x) * 4;
  const data = surface.data as Uint8ClampedArray;
  return [data[base], data[base + 1], data[base + 2], data[base + 3]];
}

function setPixel(surface: MemorySurface, x: number, y: number, color: RGBAColor): void {
  if (surface.mode === "P") {
    const index = paletteIndexForColor(surface.palette, color);
    if (index === -1) {
      throw new Error("Color not present in palette; cannot write to indexed surface.");
    }
    const data = surface.data as Uint8Array;
    const pixelIndex = y * surface.width + x;
    if (data[pixelIndex] !== index) {
      data[pixelIndex] = index;
      surface.indexedCacheDirty = true;
    }
    return;
  }
  const base = (y * surface.width + x) * 4;
  const data = surface.data as Uint8ClampedArray;
  data[base] = color[0];
  data[base + 1] = color[1];
  data[base + 2] = color[2];
  data[base + 3] = color[3];
}

function fillSurface(surface: MemorySurface, color: RGBAColor): void {
  if (surface.mode === "P") {
    const index = paletteIndexForColor(surface.palette, color);
    if (index === -1) {
      throw new Error("Color not found in palette; cannot fill indexed surface.");
    }
    (surface.data as Uint8Array).fill(index);
    surface.indexedCacheDirty = true;
    return;
  }
  const data = surface.data as Uint8ClampedArray;
  for (let i = 0; i < data.length; i += 4) {
    data[i] = color[0];
    data[i + 1] = color[1];
    data[i + 2] = color[2];
    data[i + 3] = color[3];
  }
}

function scaleSurface(surface: MemorySurface, width: number, height: number): MemorySurface {
  const scaled = createSurface(width, height, surface.mode === "P");
  const srcWidth = surface.width;
  const srcHeight = surface.height;
  if (surface.mode === "P") {
    const src = surface.data as Uint8Array;
    const dest = scaled.data as Uint8Array;
    for (let y = 0; y < height; y += 1) {
      const srcY = Math.floor((y * srcHeight) / height);
      const srcRow = srcY * srcWidth;
      const destRow = y * width;
      for (let x = 0; x < width; x += 1) {
        const srcX = Math.floor((x * srcWidth) / width);
        dest[destRow + x] = src[srcRow + srcX];
      }
    }
  } else {
    const src = surface.data as Uint8ClampedArray;
    const dest = scaled.data as Uint8ClampedArray;
    for (let y = 0; y < height; y += 1) {
      const srcY = Math.floor((y * srcHeight) / height);
      const srcRow = srcY * srcWidth;
      const destRow = y * width;
      for (let x = 0; x < width; x += 1) {
        const srcX = Math.floor((x * srcWidth) / width);
        const srcIndex = (srcRow + srcX) * 4;
        const destIndex = (destRow + x) * 4;
        dest[destIndex] = src[srcIndex];
        dest[destIndex + 1] = src[srcIndex + 1];
        dest[destIndex + 2] = src[srcIndex + 2];
        dest[destIndex + 3] = src[srcIndex + 3];
      }
    }
  }
  scaled.colorkey = surface.colorkey;
  if (surface.mode === "P") {
    scaled.palette = surface.palette.map((entry) => [...entry] as RGBAColor);
    scaled.paletteBytes = new Uint8Array(surface.paletteBytes ?? paletteBytesFromPalette(scaled.palette));
    scaled.indexedCacheDirty = true;
  }
  return scaled;
}

function blitSurface(
  dest: MemorySurface,
  src: MemorySurface,
  rect: [number, number, number, number] | null
): void {
  const dx = rect ? rect[0] : 0;
  const dy = rect ? rect[1] : 0;
  const width = rect ? Math.min(rect[2], src.width) : src.width;
  const height = rect ? Math.min(rect[3], src.height) : src.height;
  const maxWidth = Math.max(0, Math.min(width, dest.width - dx));
  const maxHeight = Math.max(0, Math.min(height, dest.height - dy));
  const startX = Math.max(0, -dx);
  const startY = Math.max(0, -dy);
  const colorkey = src.colorkey;
  const hasColorkey = Boolean(colorkey);
  const colorkeyR = colorkey?.[0] ?? 0;
  const colorkeyG = colorkey?.[1] ?? 0;
  const colorkeyB = colorkey?.[2] ?? 0;

  if (src.mode === "RGBA" && dest.mode === "RGBA") {
    const srcData = src.data as Uint8ClampedArray;
    const destData = dest.data as Uint8ClampedArray;
    for (let y = startY; y < maxHeight; y += 1) {
      const destY = dy + y;
      const srcRow = y * src.width;
      const destRow = destY * dest.width;
      for (let x = startX; x < maxWidth; x += 1) {
        const destX = dx + x;
        const srcIndex = (srcRow + x) * 4;
        const r = srcData[srcIndex];
        const g = srcData[srcIndex + 1];
        const b = srcData[srcIndex + 2];
        const a = srcData[srcIndex + 3];
        if (hasColorkey) {
          if (r === colorkeyR && g === colorkeyG && b === colorkeyB) {
            continue;
          }
        } else if (a === 0) {
          continue;
        }
        const destIndex = (destRow + destX) * 4;
        destData[destIndex] = r;
        destData[destIndex + 1] = g;
        destData[destIndex + 2] = b;
        destData[destIndex + 3] = a;
      }
    }
    return;
  }

  if (src.mode === "P" && dest.mode === "P") {
    const srcData = src.data as Uint8Array;
    const destData = dest.data as Uint8Array;
    const colorkeyIndex = hasColorkey
      ? paletteIndexForColor(src.palette, [colorkeyR, colorkeyG, colorkeyB, 255])
      : -1;
    let changed = false;
    for (let y = startY; y < maxHeight; y += 1) {
      const destY = dy + y;
      const srcRow = y * src.width;
      const destRow = destY * dest.width;
      for (let x = startX; x < maxWidth; x += 1) {
        const destX = dx + x;
        const srcIndex = srcData[srcRow + x];
        if (colorkeyIndex !== -1 && srcIndex === colorkeyIndex) {
          continue;
        }
        const destIndex = destRow + destX;
        if (destData[destIndex] !== srcIndex) {
          destData[destIndex] = srcIndex;
          changed = true;
        }
      }
    }
    if (changed) {
      dest.indexedCacheDirty = true;
    }
    return;
  }

  if (src.mode === "P" && dest.mode === "RGBA") {
    const srcData = src.data as Uint8Array;
    const destData = dest.data as Uint8ClampedArray;
    if (!src.paletteBytes) {
      src.paletteBytes = paletteBytesFromPalette(src.palette);
    }
    const srcPalette = src.paletteBytes;
    const colorkeyIndex = hasColorkey
      ? paletteIndexForColor(src.palette, [colorkeyR, colorkeyG, colorkeyB, 255])
      : -1;
    for (let y = startY; y < maxHeight; y += 1) {
      const destY = dy + y;
      const srcRow = y * src.width;
      const destRow = destY * dest.width;
      for (let x = startX; x < maxWidth; x += 1) {
        const destX = dx + x;
        const srcIndex = srcData[srcRow + x];
        if (colorkeyIndex !== -1 && srcIndex === colorkeyIndex) {
          continue;
        }
        const paletteBase = srcIndex * 4;
        const destIndex = (destRow + destX) * 4;
        destData[destIndex] = srcPalette[paletteBase];
        destData[destIndex + 1] = srcPalette[paletteBase + 1];
        destData[destIndex + 2] = srcPalette[paletteBase + 2];
        destData[destIndex + 3] = srcPalette[paletteBase + 3];
      }
    }
    return;
  }

  for (let y = startY; y < maxHeight; y += 1) {
    for (let x = startX; x < maxWidth; x += 1) {
      const color = getPixel(src, x, y);
      if (hasColorkey && color[0] === colorkeyR && color[1] === colorkeyG && color[2] === colorkeyB) {
        continue;
      }
      if (!hasColorkey && src.mode === "RGBA" && color[3] === 0) {
        continue;
      }
      setPixel(dest, dx + x, dy + y, color);
    }
  }
}

function surfaceToImageData(surface: MemorySurface): ImageData {
  const ImageDataConstructor = globalThis.ImageData;
  if (surface.mode === "RGBA") {
    const rgba = surface.data instanceof Uint8ClampedArray
      ? surface.data
      : new Uint8ClampedArray(surface.data);
    const cached = surface.imageDataCache;
    if (
      cached &&
      cached.width === surface.width &&
      cached.height === surface.height &&
      cached.data === rgba
    ) {
      return cached;
    }
    const imageData = typeof ImageDataConstructor === "function"
      ? new ImageDataConstructor(rgba as Uint8ClampedArray<ArrayBuffer>, surface.width, surface.height)
      : ({
          data: rgba,
          width: surface.width,
          height: surface.height,
          colorSpace: "srgb",
        } as ImageData);
    surface.imageDataCache = imageData;
    return imageData;
  }
  let buffer = surface.indexedRgbaCache;
  let imageData = surface.indexedImageDataCache;
  if (
    !buffer ||
    !imageData ||
    imageData.width !== surface.width ||
    imageData.height !== surface.height
  ) {
    buffer = new Uint8ClampedArray(surface.width * surface.height * 4);
    imageData = typeof ImageDataConstructor === "function"
      ? new ImageDataConstructor(buffer as Uint8ClampedArray<ArrayBuffer>, surface.width, surface.height)
      : ({
          data: buffer,
          width: surface.width,
          height: surface.height,
          colorSpace: "srgb",
        } as ImageData);
    surface.indexedRgbaCache = buffer;
    surface.indexedImageDataCache = imageData;
    surface.indexedCacheDirty = true;
  }

  if (surface.indexedCacheDirty) {
    const indexed = surface.data as Uint8Array;
    if (!surface.paletteBytes) {
      surface.paletteBytes = paletteBytesFromPalette(surface.palette);
    }
    const paletteBytes = surface.paletteBytes;
    for (let i = 0; i < indexed.length; i += 1) {
      const paletteBase = indexed[i] * 4;
      const base = i * 4;
      buffer[base] = paletteBytes[paletteBase];
      buffer[base + 1] = paletteBytes[paletteBase + 1];
      buffer[base + 2] = paletteBytes[paletteBase + 2];
      buffer[base + 3] = paletteBytes[paletteBase + 3];
    }
    surface.indexedCacheDirty = false;
  }
  return imageData;
}

function normalizePngFilename(path: string): string {
  const sanitized = path.trim() || "frame.png";
  const base = sanitized.split(/[\\/]/).filter(Boolean).pop() || "frame";
  const withoutQuery = base.split("?")[0];
  return withoutQuery.toLowerCase().endsWith(".png") ? withoutQuery : `${withoutQuery}.png`;
}

function imageDataFromMemorySurface(surface: MemorySurface): ImageData {
  const rgba = rgbaBytesFromSurface(surface);
  const data = new Uint8ClampedArray(rgba);
  const ImageDataConstructor = globalThis.ImageData;
  if (typeof ImageDataConstructor === "function") {
    return new ImageDataConstructor(data, surface.width, surface.height);
  }
  return {
    data,
    width: surface.width,
    height: surface.height,
    colorSpace: "srgb",
  } as ImageData;
}

async function pngUrlFromCanvas(
  canvas: HTMLCanvasElement | OffscreenCanvas
): Promise<string> {
  const offscreenCanvas = canvas as OffscreenCanvas;
  if (typeof offscreenCanvas.convertToBlob === "function") {
    const blob = await offscreenCanvas.convertToBlob({ type: "image/png" });
    if (!blob) {
      throw new Error("Failed to encode PNG from canvas.");
    }
    if (typeof URL === "undefined" || typeof URL.createObjectURL !== "function") {
      throw new Error("PNG save requires URL.createObjectURL for blob downloads.");
    }
    return URL.createObjectURL(blob);
  }

  const htmlCanvas = canvas as HTMLCanvasElement;
  if (typeof htmlCanvas.toDataURL !== "function") {
    throw new Error("Canvas is missing PNG export methods.");
  }
  return htmlCanvas.toDataURL("image/png");
}

function rgbaBytesFromSurface(surface: MemorySurface): Uint8Array {
  if (surface.mode === "RGBA") {
    const data = surface.data as Uint8ClampedArray;
    return new Uint8Array(data.buffer, data.byteOffset, data.byteLength);
  }
  const buffer = new Uint8Array(surface.width * surface.height * 4);
  const indexed = surface.data as Uint8Array;
  for (let i = 0; i < indexed.length; i += 1) {
    const color = surface.palette[indexed[i]];
    const base = i * 4;
    buffer[base] = color[0];
    buffer[base + 1] = color[1];
    buffer[base + 2] = color[2];
    buffer[base + 3] = color[3];
  }
  return buffer;
}

function decodeFormat(
  payload: Uint8Array,
  width: number,
  height: number,
  format: BufferFormat
): MemorySurface {
  if (format === "P") {
    const surface = createSurface(width, height, true);
    if (payload.length !== width * height) {
      throw new Error("Palettized payload size does not match dimensions.");
    }
    (surface.data as Uint8Array).set(payload);
    return surface;
  }
  const expected = width * height * (format === "RGB" || format === "BGR" ? 3 : 4);
  if (payload.length !== expected) {
    throw new Error("Pixel payload size does not match dimensions.");
  }
  const surface = createSurface(width, height, false);
  const data = surface.data as Uint8ClampedArray;
  for (let i = 0; i < width * height; i += 1) {
    const base = i * (format === "RGB" || format === "BGR" ? 3 : 4);
    const out = i * 4;
    switch (format) {
      case "RGB":
        data[out] = payload[base];
        data[out + 1] = payload[base + 1];
        data[out + 2] = payload[base + 2];
        data[out + 3] = 255;
        break;
      case "BGR":
        data[out] = payload[base + 2];
        data[out + 1] = payload[base + 1];
        data[out + 2] = payload[base];
        data[out + 3] = 255;
        break;
      case "RGBA":
        data[out] = payload[base];
        data[out + 1] = payload[base + 1];
        data[out + 2] = payload[base + 2];
        data[out + 3] = payload[base + 3];
        break;
      case "RGBX":
        data[out] = payload[base];
        data[out + 1] = payload[base + 1];
        data[out + 2] = payload[base + 2];
        data[out + 3] = 255;
        break;
      case "BGRA":
        data[out] = payload[base + 2];
        data[out + 1] = payload[base + 1];
        data[out + 2] = payload[base];
        data[out + 3] = payload[base + 3];
        break;
      case "ARGB":
        data[out] = payload[base + 1];
        data[out + 1] = payload[base + 2];
        data[out + 2] = payload[base + 3];
        data[out + 3] = payload[base];
        break;
    }
  }
  return surface;
}

function encodeFormat(surface: MemorySurface, format: ToBytesFormat): Uint8Array {
  if (format === "P" && surface.mode !== "P") {
    throw new Error("Cannot export non-indexed surface as palettized buffer.");
  }
  if (format === "P") {
    return new Uint8Array(surface.data as Uint8Array);
  }

  const rgba = rgbaBytesFromSurface(surface);
  const result =
    format === "RGB" || format === "BGR"
      ? new Uint8Array(surface.width * surface.height * 3)
      : new Uint8Array(surface.width * surface.height * 4);

  for (let i = 0; i < surface.width * surface.height; i += 1) {
    const base = i * 4;
    const out = i * (format === "RGB" || format === "BGR" ? 3 : 4);
    const r = rgba[base];
    const g = rgba[base + 1];
    const b = rgba[base + 2];
    const a = rgba[base + 3];
    switch (format) {
      case "RGB":
        result[out] = r;
        result[out + 1] = g;
        result[out + 2] = b;
        break;
      case "BGR":
        result[out] = b;
        result[out + 1] = g;
        result[out + 2] = r;
        break;
      case "RGBA":
        result[out] = r;
        result[out + 1] = g;
        result[out + 2] = b;
        result[out + 3] = a;
        break;
      case "RGBX":
        result[out] = r;
        result[out + 1] = g;
        result[out + 2] = b;
        result[out + 3] = 0;
        break;
      case "BGRA":
        result[out] = b;
        result[out + 1] = g;
        result[out + 2] = r;
        result[out + 3] = a;
        break;
      case "ARGB":
        result[out] = a;
        result[out + 1] = r;
        result[out + 2] = g;
        result[out + 3] = b;
        break;
    }
  }
  return result;
}

async function loadImageData(path: string): Promise<ImageData> {
  if (typeof fetch !== "function" || typeof createImageBitmap !== "function") {
    throw new Error("Image loading requires fetch and createImageBitmap.");
  }
  const response = await fetch(path);
  if (!response.ok) {
    throw new Error(`Failed to load image: ${path}`);
  }
  const blob = await response.blob();
  const bitmap = await createImageBitmap(blob);
  const canvas =
    typeof OffscreenCanvas !== "undefined"
      ? new OffscreenCanvas(bitmap.width, bitmap.height)
      : typeof document !== "undefined"
        ? document.createElement("canvas")
        : null;
  if (!canvas) {
    throw new Error("No canvas available for image decode.");
  }
  canvas.width = bitmap.width;
  canvas.height = bitmap.height;
  const ctx = canvas.getContext("2d") as CanvasRenderingContext2D | OffscreenCanvasRenderingContext2D | null;
  if (!ctx) {
    throw new Error("Unable to access 2D canvas context.");
  }
  ctx.drawImage(bitmap, 0, 0);
  return ctx.getImageData(0, 0, bitmap.width, bitmap.height);
}

export class WebBackend implements BackendAdapter {
  name = "web";
  private readonly headless: boolean;
  private readonly inputEnabled: boolean;
  private readonly eventQueue: BackendEvent[] = [];
  private readonly keyDownHandler?: (event: KeyboardEvent) => void;
  private readonly keyUpHandler?: (event: KeyboardEvent) => void;
  private readonly startTime: number;

  constructor(options?: WebBackendOptions) {
    this.headless = options?.headless ?? false;
    this.inputEnabled = options?.inputEnabled ?? !this.headless;
    this.startTime = typeof performance !== "undefined" ? performance.now() : Date.now();

    if (this.inputEnabled && typeof window !== "undefined") {
      this.keyDownHandler = (event: KeyboardEvent) => {
        const payload = {
          type: KEYDOWN,
          key: event.keyCode ?? null,
          unicode: event.key.length === 1 ? event.key : null,
        };
        this.enqueueEvent(payload);
      };
      this.keyUpHandler = (event: KeyboardEvent) => {
        const payload = {
          type: KEYUP,
          key: event.keyCode ?? null,
          unicode: event.key.length === 1 ? event.key : null,
        };
        this.enqueueEvent(payload);
      };
      window.addEventListener("keydown", this.keyDownHandler);
      window.addEventListener("keyup", this.keyUpHandler);
    }
  }

  close(): void {
    if (typeof window !== "undefined") {
      if (this.keyDownHandler) {
        window.removeEventListener("keydown", this.keyDownHandler);
      }
      if (this.keyUpHandler) {
        window.removeEventListener("keyup", this.keyUpHandler);
      }
    }
  }

  createSurface(width: number, height: number, options?: { indexed?: boolean }): BackendSurface {
    const surface = createSurface(width, height, options?.indexed ?? false);
    return new BackendSurface(this, surface);
  }

  surfaceFromBytes(
    payload: Uint8Array,
    width: number,
    height: number,
    format: BufferFormat
  ): BackendSurface {
    const parsed = BufferFormatSchema.parse(format);
    const surface = decodeFormat(payload, width, height, parsed);
    return new BackendSurface(this, surface);
  }

  surfaceToBytes(surface: BackendSurface, format: ToBytesFormat): Uint8Array {
    if (format === "RGBA_PREMULT" || format === "ARGB_PREMULT") {
      const raw = assertSurface(surface);
      const rgba = rgbaBytesFromSurface(raw);
      const output = new Uint8Array(rgba.length);
      for (let i = 0; i < rgba.length; i += 4) {
        const r = rgba[i];
        const g = rgba[i + 1];
        const b = rgba[i + 2];
        const a = rgba[i + 3];
        const pr = Math.round((r * a) / 255);
        const pg = Math.round((g * a) / 255);
        const pb = Math.round((b * a) / 255);
        if (format === "ARGB_PREMULT") {
          output[i] = a;
          output[i + 1] = pr;
          output[i + 2] = pg;
          output[i + 3] = pb;
        } else {
          output[i] = pr;
          output[i + 1] = pg;
          output[i + 2] = pb;
          output[i + 3] = a;
        }
      }
      return output;
    }
    const parsed = ToBytesFormatSchema.parse(format);
    return encodeFormat(assertSurface(surface), parsed);
  }

  async loadPng(path: string): Promise<BackendSurface> {
    const imageData = await loadImageData(path);
    const surface = createSurface(imageData.width, imageData.height, false);
    (surface.data as Uint8ClampedArray).set(imageData.data);
    return new BackendSurface(this, surface);
  }

  async savePng(_surface: BackendSurface, _path: string): Promise<void> {
    const raw = assertSurface(_surface);
    const pngName = normalizePngFilename(_path);
    const canvas = ensureCanvas(raw.width, raw.height, true);
    const context = getCanvasContext(canvas);
    const imageData = imageDataFromMemorySurface(raw);
    context.putImageData(imageData, 0, 0);

    let url: string | null = null;
    let needsRevoke = false;
    try {
      url = await pngUrlFromCanvas(canvas);
      needsRevoke = url.startsWith("blob:");
      if (typeof document === "undefined") {
        throw new Error(
          "PNG save in the web backend requires a document environment."
        );
      }
      const anchor = document.createElement("a");
      anchor.href = url;
      anchor.download = pngName;
      anchor.rel = "noopener";
      anchor.style.display = "none";
      if (typeof document.body?.appendChild === "function") {
        document.body.appendChild(anchor);
      }
      anchor.click();
      if (typeof document.body?.removeChild === "function") {
        document.body.removeChild(anchor);
      }
    } finally {
      if (
        needsRevoke &&
        url !== null &&
        typeof URL !== "undefined" &&
        typeof URL.revokeObjectURL === "function"
      ) {
        URL.revokeObjectURL(url);
      }
    }
  }

  blit(
    dest: BackendSurface,
    src: BackendSurface,
    rect: [number, number, number, number] | null
  ): void {
    blitSurface(assertSurface(dest), assertSurface(src), rect);
  }

  fill(surface: BackendSurface, color: RGBAColor): void {
    fillSurface(assertSurface(surface), clampColor(color));
  }

  scale(surface: BackendSurface, size: [number, number]): BackendSurface {
    const scaled = scaleSurface(assertSurface(surface), size[0], size[1]);
    return new BackendSurface(this, scaled);
  }

  setColorkey(surface: BackendSurface, color: RGBAColor | null): void {
    const raw = assertSurface(surface);
    raw.colorkey = color ? clampColor(color) : null;
  }

  setPalette(surface: BackendSurface, palette: ReadonlyArray<ReadonlyArray<number>>): void {
    const raw = assertSurface(surface);
    if (raw.mode !== "P") {
      throw new Error("Palette can only be set on indexed (P) surfaces.");
    }
    raw.palette = ensurePalette(palette);
    raw.paletteBytes = paletteBytesFromPalette(raw.palette);
    raw.indexedCacheDirty = true;
  }

  getPalette(surface: BackendSurface): RGBAColor[] {
    const raw = assertSurface(surface);
    if (raw.mode !== "P") {
      return [];
    }
    return raw.palette.map((entry) => [...entry] as RGBAColor);
  }

  getPixel(surface: BackendSurface, x: number, y: number): RGBAColor {
    return getPixel(assertSurface(surface), x, y);
  }

  setPixel(surface: BackendSurface, x: number, y: number, color: RGBAColor): void {
    setPixel(assertSurface(surface), x, y, clampColor(color));
  }

  getSize(surface: BackendSurface): [number, number] {
    const raw = assertSurface(surface);
    return [raw.width, raw.height];
  }

  createWindow(
    width: number,
    height: number,
    options?: { scale?: number; headless?: boolean }
  ): BackendWindow {
    const scale = options?.scale ?? 1;
    const headless = options?.headless ?? this.headless;
    const canvas = ensureCanvas(width * scale, height * scale, headless);
    const context = getCanvasContext(canvas);
    (context as CanvasRenderingContext2D).imageSmoothingEnabled = false;
    const window: CanvasWindow = {
      canvas,
      context,
      width,
      height,
      scale,
      headless,
    };
    return new BackendWindow(this, window);
  }

  present(window: BackendWindow, surface: BackendSurface): void {
    const raw = window.raw as CanvasWindow;
    if (!raw || raw.headless) {
      return;
    }
    const imageData = surfaceToImageData(surface.raw as MemorySurface);
    const context = raw.context;
    const targetWidth = raw.width * raw.scale;
    const targetHeight = raw.height * raw.scale;

    if (raw.scale === 1) {
      context.putImageData(imageData, 0, 0);
      return;
    }

    let scratch = raw.scratch;
    let scratchCtx = raw.scratchContext;
    if (!scratch || !scratchCtx || scratch.width !== raw.width || scratch.height !== raw.height) {
      scratch = ensureCanvas(raw.width, raw.height, true);
      scratchCtx = getCanvasContext(scratch);
      (scratchCtx as CanvasRenderingContext2D).imageSmoothingEnabled = false;
      raw.scratch = scratch;
      raw.scratchContext = scratchCtx;
    }
    scratchCtx.putImageData(imageData, 0, 0);
    context.clearRect(0, 0, targetWidth, targetHeight);
    context.drawImage(scratch as CanvasImageSource, 0, 0, targetWidth, targetHeight);
  }

  pollEvents(_window?: BackendWindow | null): BackendEvent[] {
    const events = [...this.eventQueue];
    this.eventQueue.length = 0;
    return events;
  }

  ticks(): number {
    const now = typeof performance !== "undefined" ? performance.now() : Date.now();
    return Math.floor(now - this.startTime);
  }

  private enqueueEvent(event: BackendEvent): void {
    this.eventQueue.push(event);
  }
}
