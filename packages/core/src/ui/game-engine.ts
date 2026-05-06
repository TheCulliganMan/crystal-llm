// A minimal polyfill for the Pygame API using HTML5 Canvas.

import { HeadlessCanvas, HeadlessContext2D } from "./headless-canvas";
import type { SurfaceLike } from "./font-renderer";
import { toPublicAssetUrl } from "@pokecrystal/core/core/asset-manifest";
import { GB_FRAME_RATE } from "@pokecrystal/core/core/gb-timing";

const SRCALPHA = 1;
const getTimestamp: () => number =
  typeof performance !== "undefined" && typeof performance.now === "function"
    ? () => performance.now()
    : () => Date.now();

type CanvasContext =
  | CanvasRenderingContext2D
  | OffscreenCanvasRenderingContext2D
  | HeadlessContext2D;

type CanvasSurface = HTMLCanvasElement | OffscreenCanvas | HeadlessCanvas;

const canUseOffscreen = typeof window !== "undefined" && typeof OffscreenCanvas !== "undefined";
const canUseDocument = typeof document !== "undefined";

const ensureCanvasImageSource = (canvas: CanvasSurface): CanvasImageSource => {
  if (canvas instanceof HeadlessCanvas) {
    throw new Error("Headless canvases cannot be used as drawImage sources.");
  }
  return canvas;
};

const isHeadlessCanvas = (canvas: CanvasSurface): canvas is HeadlessCanvas =>
  canvas instanceof HeadlessCanvas;

export class Rect {
  public x: number;
  public y: number;
  public width: number;
  public height: number;

  constructor(x: number, y: number, width: number, height: number) {
    this.x = x;
    this.y = y;
    this.width = width;
    this.height = height;
  }

  get left() {
    return this.x;
  }
  get top() {
    return this.y;
  }
  get right() {
    return this.x + this.width;
  }
  get bottom() {
    return this.y + this.height;
  }
}

export class Surface {
  public canvas: CanvasSurface;
  protected context: CanvasContext;
  public colorkey: [number, number, number] | null = null;

  constructor(width: number, height: number) {
    if (canUseOffscreen) {
      this.canvas = new OffscreenCanvas(width, height);
    } else if (canUseDocument) {
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      this.canvas = canvas;
    } else {
      this.canvas = new HeadlessCanvas(width, height);
    }
    let context = this.canvas.getContext("2d", { willReadFrequently: true }) as
      | CanvasContext
      | null;
    if (!context) {
      context = this.canvas.getContext("2d") as CanvasContext | null;
    }
    if (!context) {
      throw new Error("Failed to get 2D rendering context");
    }
    this.context = context as CanvasContext;
  }

  get_width(): number {
    return this.canvas.width;
  }

  get_height(): number {
    return this.canvas.height;
  }

  get_size(): [number, number] {
    return [this.canvas.width, this.canvas.height];
  }

  getCanvasImageSource(): CanvasImageSource | null {
    if (this.canvas instanceof HeadlessCanvas) {
      return this.canvas as unknown as CanvasImageSource;
    }
    return this.canvas as CanvasImageSource;
  }

  static fromImageData(imageData: ImageData): Surface {
    const surface = new Surface(imageData.width, imageData.height);
    surface.getContext()?.putImageData(imageData, 0, 0);
    return surface;
  }

  get width(): number {
    return this.canvas.width;
  }

  get height(): number {
    return this.canvas.height;
  }

  blit(
    source: Surface | SurfaceLike,
    dest: [number, number] | { x: number; y: number },
    area?: { x: number; y: number; width: number; height: number }
  ) {
    const destX = Array.isArray(dest) ? dest[0] : dest.x;
    const destY = Array.isArray(dest) ? dest[1] : dest.y;
    this.blitAt(source, destX, destY, area);
  }

  blitAt(
    source: Surface | SurfaceLike,
    destX: number,
    destY: number,
    area?: { x: number; y: number; width: number; height: number }
  ) {
    const srcCanvas = (source as Surface).getCanvasImageSource();
    if (!srcCanvas) {
      return;
    }
    if (area) {
      this.context.drawImage(
        srcCanvas,
        area.x,
        area.y,
        area.width,
        area.height,
        destX,
        destY,
        area.width,
        area.height
      );
      return;
    }
    this.context.drawImage(srcCanvas, destX, destY);
  }

  fill(
    color: [number, number, number, number],
    rect?: { x: number; y: number; width: number; height: number }
  ) {
    this.context.fillStyle = `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${
      color[3] / 255
    })`;
    if (rect) {
      this.context.fillRect(rect.x, rect.y, rect.width, rect.height);
    } else {
      this.context.fillRect(0, 0, this.canvas.width, this.canvas.height);
    }
  }

  subsurface(rect: Rect): Surface {
    const newSurface = new Surface(rect.width, rect.height);
    newSurface.blit(this, [0, 0], rect);
    return newSurface;
  }

  get_at(pos: [number, number]): [number, number, number, number] {
    const [x, y] = pos;
    const pixel = this.context.getImageData(x, y, 1, 1).data;
    return [pixel[0], pixel[1], pixel[2], pixel[3]];
  }

  set_at(pos: [number, number], color: [number, number, number, number]) {
    const [x, y] = pos;
    this.context.fillStyle = `rgba(${color[0]}, ${color[1]}, ${color[2]}, ${
      color[3] / 255
    })`;
    this.context.fillRect(x, y, 1, 1);
  }

  getAt(x: number, y: number): [number, number, number, number] {
    return this.get_at([x, y]);
  }

  setAt(x: number, y: number, color: [number, number, number, number]): void {
    this.set_at([x, y], color);
  }

  getImageData(): ImageData {
    return this.context.getImageData(0, 0, this.canvas.width, this.canvas.height);
  }

  getContext(): CanvasContext {
    return this.context;
  }

  copy(): Surface {
    const newSurface = new Surface(this.canvas.width, this.canvas.height);
    newSurface.blit(this, [0, 0]);
    return newSurface;
  }

  set_colorkey(color: [number, number, number]): void {
    this.colorkey = color;
    const image = this.context.getImageData(0, 0, this.canvas.width, this.canvas.height);
    const data = image.data;
    const [r, g, b] = color;
    for (let i = 0; i < data.length; i += 4) {
      if (data[i] === r && data[i + 1] === g && data[i + 2] === b) {
        data[i + 3] = 0;
      }
    }
    this.context.putImageData(image, 0, 0);
  }

  get_flags(): number {
    return SRCALPHA;
  }

  convert(): this {
    // In a browser context, surfaces are generally fine as-is.
    return this;
  }

  convert_alpha(): this {
    // In a browser context, surfaces are generally fine as-is.
    return this;
  }
}

const PNG_SIGNATURE = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

type PngHeader = {
  width: number;
  height: number;
  bitDepth: number;
  colorType: number;
  compression: number;
  filter: number;
  interlace: number;
};

const parsePngSize = (data: Uint8Array): [number, number] | null => {
  if (data.length < 24) {
    return null;
  }
  for (let i = 0; i < PNG_SIGNATURE.length; i += 1) {
    if (data[i] !== PNG_SIGNATURE[i]) {
      return null;
    }
  }
  const chunkType = String.fromCharCode(
    data[12],
    data[13],
    data[14],
    data[15]
  );
  if (chunkType !== "IHDR") {
    return null;
  }
  const width =
    (data[16] << 24) |
    (data[17] << 16) |
    (data[18] << 8) |
    data[19];
  const height =
    (data[20] << 24) |
    (data[21] << 16) |
    (data[22] << 8) |
    data[23];
  if (width <= 0 || height <= 0) {
    return null;
  }
  return [width >>> 0, height >>> 0];
};

const readUInt32BE = (data: Uint8Array, offset: number): number => {
  if (offset + 3 >= data.length) {
    throw new Error("PNG chunk length is truncated.");
  }
  return (
    (data[offset] << 24) |
    (data[offset + 1] << 16) |
    (data[offset + 2] << 8) |
    data[offset + 3]
  ) >>> 0;
};

const readChunkType = (data: Uint8Array, offset: number): string => {
  if (offset + 3 >= data.length) {
    throw new Error("PNG chunk type is truncated.");
  }
  return String.fromCharCode(
    data[offset],
    data[offset + 1],
    data[offset + 2],
    data[offset + 3]
  );
};

const paethPredictor = (a: number, b: number, c: number): number => {
  const p = a + b - c;
  const pa = Math.abs(p - a);
  const pb = Math.abs(p - b);
  const pc = Math.abs(p - c);
  if (pa <= pb && pa <= pc) {
    return a;
  }
  if (pb <= pc) {
    return b;
  }
  return c;
};

const unfilterScanlines = (
  data: Uint8Array,
  width: number,
  height: number,
  rowLength: number,
  bytesPerPixel: number
): Uint8Array => {
  const output = new Uint8Array(rowLength * height);
  let inputOffset = 0;
  let outputOffset = 0;
  let previousRow = new Uint8Array(rowLength);
  for (let row = 0; row < height; row += 1) {
    if (inputOffset >= data.length) {
      throw new Error("PNG scanlines are truncated.");
    }
    const filterType = data[inputOffset];
    inputOffset += 1;
    const rawRow = data.subarray(inputOffset, inputOffset + rowLength);
    if (rawRow.length < rowLength) {
      throw new Error("PNG scanline payload is truncated.");
    }
    const outRow = output.subarray(outputOffset, outputOffset + rowLength);
    switch (filterType) {
      case 0:
        outRow.set(rawRow);
        break;
      case 1:
        for (let i = 0; i < rowLength; i += 1) {
          const left = i >= bytesPerPixel ? outRow[i - bytesPerPixel] : 0;
          outRow[i] = (rawRow[i] + left) & 0xff;
        }
        break;
      case 2:
        for (let i = 0; i < rowLength; i += 1) {
          const up = previousRow[i] ?? 0;
          outRow[i] = (rawRow[i] + up) & 0xff;
        }
        break;
      case 3:
        for (let i = 0; i < rowLength; i += 1) {
          const left = i >= bytesPerPixel ? outRow[i - bytesPerPixel] : 0;
          const up = previousRow[i] ?? 0;
          outRow[i] = (rawRow[i] + Math.floor((left + up) / 2)) & 0xff;
        }
        break;
      case 4:
        for (let i = 0; i < rowLength; i += 1) {
          const left = i >= bytesPerPixel ? outRow[i - bytesPerPixel] : 0;
          const up = previousRow[i] ?? 0;
          const upLeft = i >= bytesPerPixel ? previousRow[i - bytesPerPixel] : 0;
          outRow[i] = (rawRow[i] + paethPredictor(left, up, upLeft)) & 0xff;
        }
        break;
      default:
        throw new Error(`Unsupported PNG filter type ${filterType}.`);
    }
    inputOffset += rowLength;
    outputOffset += rowLength;
    previousRow = outRow;
  }
  return output;
};

const decodePngPixels = (
  raw: Uint8Array,
  inflate: (payload: Uint8Array) => Uint8Array
): { header: PngHeader; rgba: Uint8ClampedArray } => {
  for (let i = 0; i < PNG_SIGNATURE.length; i += 1) {
    if (raw[i] !== PNG_SIGNATURE[i]) {
      throw new Error("Invalid PNG signature.");
    }
  }
  let header: PngHeader | null = null;
  let palette: Uint8Array | null = null;
  let transparency: Uint8Array | null = null;
  const idat: Uint8Array[] = [];
  let offset = PNG_SIGNATURE.length;
  while (offset + 8 <= raw.length) {
    const length = readUInt32BE(raw, offset);
    offset += 4;
    const type = readChunkType(raw, offset);
    offset += 4;
    const chunkEnd = offset + length;
    if (chunkEnd > raw.length) {
      throw new Error(`PNG chunk ${type} is truncated.`);
    }
    const chunk = raw.subarray(offset, chunkEnd);
    offset = chunkEnd + 4;
    if (type === "IHDR") {
      if (chunk.length < 13) {
        throw new Error("PNG IHDR is truncated.");
      }
      header = {
        width:
          (chunk[0] << 24) |
          (chunk[1] << 16) |
          (chunk[2] << 8) |
          chunk[3],
        height:
          (chunk[4] << 24) |
          (chunk[5] << 16) |
          (chunk[6] << 8) |
          chunk[7],
        bitDepth: chunk[8],
        colorType: chunk[9],
        compression: chunk[10],
        filter: chunk[11],
        interlace: chunk[12],
      };
    } else if (type === "PLTE") {
      palette = chunk;
    } else if (type === "tRNS") {
      transparency = chunk;
    } else if (type === "IDAT") {
      idat.push(chunk);
    } else if (type === "IEND") {
      break;
    }
  }
  if (!header) {
    throw new Error("PNG missing IHDR chunk.");
  }
  if (header.compression !== 0 || header.filter !== 0) {
    throw new Error("Unsupported PNG compression or filter method.");
  }
  if (header.interlace !== 0) {
    throw new Error("Interlaced PNGs are not supported.");
  }
  if (!idat.length) {
    throw new Error("PNG missing IDAT data.");
  }

  const channels =
    header.colorType === 0
      ? 1
      : header.colorType === 2
        ? 3
        : header.colorType === 3
          ? 1
          : header.colorType === 4
            ? 2
            : header.colorType === 6
              ? 4
              : 0;
  if (!channels) {
    throw new Error(`Unsupported PNG color type ${header.colorType}.`);
  }
  if (
    header.colorType === 0 &&
    header.bitDepth !== 2 &&
    header.bitDepth !== 8
  ) {
    throw new Error(
      `Unsupported grayscale bit depth ${header.bitDepth}.`
    );
  }
  if (header.colorType === 3 && header.bitDepth !== 8) {
    throw new Error(
      `Unsupported indexed bit depth ${header.bitDepth}.`
    );
  }
  if (
    header.colorType === 6 &&
    header.bitDepth !== 8
  ) {
    throw new Error(
      `Unsupported RGBA bit depth ${header.bitDepth}.`
    );
  }
  if (header.colorType === 3 && !palette) {
    throw new Error("Indexed PNG missing palette.");
  }

  const rowLength = Math.ceil((header.width * channels * header.bitDepth) / 8);
  const bytesPerPixel = Math.max(1, Math.ceil((channels * header.bitDepth) / 8));
  const compressed = Buffer.concat(idat.map((chunk) => Buffer.from(chunk)));
  const inflated = inflate(compressed);
  const scanlineLength = (rowLength + 1) * header.height;
  if (inflated.length < scanlineLength) {
    throw new Error("PNG IDAT payload is truncated.");
  }
  const unfiltered = unfilterScanlines(
    inflated,
    header.width,
    header.height,
    rowLength,
    bytesPerPixel
  );
  const rgba = new Uint8ClampedArray(header.width * header.height * 4);
  const maxSample = header.bitDepth === 0 ? 0 : (1 << header.bitDepth) - 1;
  const samplesPerByte = Math.max(1, Math.floor(8 / header.bitDepth));
  for (let y = 0; y < header.height; y += 1) {
    const rowStart = y * rowLength;
    for (let x = 0; x < header.width; x += 1) {
      const destIndex = (y * header.width + x) * 4;
      if (header.colorType === 0) {
        const byteIndex = rowStart + Math.floor(x / samplesPerByte);
        const shift =
          8 - header.bitDepth - (x % samplesPerByte) * header.bitDepth;
        const sample = (unfiltered[byteIndex] >> shift) & maxSample;
        const value = Math.round((sample / maxSample) * 255);
        rgba[destIndex] = value;
        rgba[destIndex + 1] = value;
        rgba[destIndex + 2] = value;
        rgba[destIndex + 3] = 255;
        continue;
      }
      if (header.colorType === 3) {
        const index = unfiltered[rowStart + x] ?? 0;
        const paletteIndex = index * 3;
        const r = palette![paletteIndex] ?? 0;
        const g = palette![paletteIndex + 1] ?? 0;
        const b = palette![paletteIndex + 2] ?? 0;
        const a =
          transparency && index < transparency.length
            ? transparency[index] ?? 255
            : 255;
        rgba[destIndex] = r;
        rgba[destIndex + 1] = g;
        rgba[destIndex + 2] = b;
        rgba[destIndex + 3] = a;
        continue;
      }
      if (header.colorType === 6) {
        const offset = rowStart + x * 4;
        rgba[destIndex] = unfiltered[offset] ?? 0;
        rgba[destIndex + 1] = unfiltered[offset + 1] ?? 0;
        rgba[destIndex + 2] = unfiltered[offset + 2] ?? 0;
        rgba[destIndex + 3] = unfiltered[offset + 3] ?? 255;
        continue;
      }
      throw new Error(
        `Unsupported PNG color type ${header.colorType}.`
      );
    }
  }
  return { header, rgba };
};

const decodePngSurface = (
  raw: Uint8Array,
  inflate: (payload: Uint8Array) => Uint8Array
): Surface => {
  const { header, rgba } = decodePngPixels(raw, inflate);
  const surface = new Surface(header.width, header.height);
  const ctx = surface.getContext();
  if (!ctx) {
    throw new Error("Failed to get 2D context for PNG decode.");
  }
  const image = ctx.createImageData(header.width, header.height);
  image.data.set(rgba);
  ctx.putImageData(image, 0, 0);
  return surface;
};

const imageCache = new Map<string, Surface>();
const imageLoadPromises = new Map<string, Promise<Surface>>();
const ASSET_CACHE_NAME = "pokecrystal-assets-v1";

const isSurfaceUsable = (surface: Surface | undefined): surface is Surface => {
  if (!surface) {
    return false;
  }
  return (
    typeof surface.get_width === "function" &&
    typeof surface.get_height === "function" &&
    surface.get_width() > 0 &&
    surface.get_height() > 0
  );
};

const fetchWithCache = async (path: string): Promise<Response> => {
  const requestPath = typeof window !== "undefined" ? toPublicAssetUrl(path) : path;
  if (typeof fetch !== "function") {
    throw new Error(`Image loading requires fetch or fs access: ${requestPath}`);
  }
  if (typeof caches === "undefined") {
    return fetch(requestPath);
  }
  try {
    const cache = await caches.open(ASSET_CACHE_NAME);
    const cached = await cache.match(requestPath);
    if (cached) {
      return cached;
    }
    const response = await fetch(requestPath);
    if (response.ok) {
      await cache.put(requestPath, response.clone());
    }
    return response;
  } catch {
    return fetch(requestPath);
  }
};

async function loadImage(path: string): Promise<Surface> {
  const cached = imageCache.get(path);
  if (isSurfaceUsable(cached)) {
    return cached;
  }
  const inflight = imageLoadPromises.get(path);
  if (inflight) {
    return inflight;
  }
  const loadPromise = (async () => {
    if (typeof window !== "undefined" && typeof createImageBitmap === "function") {
      const response = await fetchWithCache(path);
      if (!response.ok) {
        throw new Error(`Failed to load image: ${path}`);
      }
      const blob = await response.blob();
      const imageBitmap = await createImageBitmap(blob);
      const surface = new Surface(imageBitmap.width, imageBitmap.height);
      surface.getContext()!.drawImage(imageBitmap, 0, 0);
      imageCache.set(path, surface);
      return surface;
    }
    if (typeof Buffer !== "undefined") {
      const { promises: fs } = await import("fs");
      const raw = await fs.readFile(path);
      const { inflateSync } = await import("zlib");
      const surface = decodePngSurface(raw, (payload) => inflateSync(payload));
      imageCache.set(path, surface);
      return surface;
    }
    throw new Error(`Image loading requires fetch or fs access: ${path}`);
  })();
  imageLoadPromises.set(path, loadPromise);
  try {
    return await loadPromise;
  } finally {
    imageLoadPromises.delete(path);
  }
}

async function preloadImage(path: string): Promise<Surface> {
  const cached = imageCache.get(path);
  if (isSurfaceUsable(cached)) {
    return cached;
  }
  const surface = await loadImage(path);
  return surface;
}

export function loadImageSync(path: string): Surface | null {
  const cached = imageCache.get(path);
  if (isSurfaceUsable(cached)) {
    return cached;
  }
  if (typeof window !== "undefined") {
    return null;
  }
  if (typeof Buffer !== "undefined") {
    try {
      const { readFileSync } = require("fs") as typeof import("fs");
      const raw = readFileSync(path);
      const { inflateSync } = require("zlib") as typeof import("zlib");
      const surface = decodePngSurface(raw, (payload) => inflateSync(payload));
      imageCache.set(path, surface);
      return surface;
    } catch {
      return null;
    }
  }
  return null;
}

export type GameEngineImageLoader = {
  load: (path: string) => Surface | Promise<Surface>;
  preload: (path: string) => Promise<Surface>;
  loadSync?: (path: string) => Surface | null;
};

const imageLoader: GameEngineImageLoader = {
  load: loadImage,
  preload: preloadImage,
  loadSync: loadImageSync,
};

export type GameEngineEvent = {
  type: string | number;
  key?: string | number | null;
  code?: string | number | null;
  direction?: string | null;
  button?: string | null;
  is_press?: boolean | null;
  text?: string | null;
  unicode?: string | null;
};

export class EngineEvent implements GameEngineEvent {
  public type: string | number;
  public key?: string | number | null;
  public code?: string | number | null;
  public direction?: string | null;
  public button?: string | null;
  public is_press?: boolean | null;
  public text?: string | null;
  public unicode?: string | null;

  constructor(type: string | number, opts?: Omit<GameEngineEvent, "type">) {
    this.type = type;
    if (opts) {
      Object.assign(this, opts);
    }
  }
}

class Clock {
  private lastTimestamp: number | null = null;

  tick(fps: number): void {
    const normalizedFps = normalizeClockFps(fps);
    if (normalizedFps <= 0) {
      this.lastTimestamp = null;
      return;
    }
    const frameDuration = 1000 / normalizedFps;
    const now = getTimestamp();
    if (this.lastTimestamp !== null) {
      const elapsed = now - this.lastTimestamp;
      const remaining = frameDuration - elapsed;
      if (remaining > 0) {
        const target = now + remaining;
        while (getTimestamp() < target) {
          // intentionally busy-wait to keep the legacy tick cadence
        }
      }
    }
    this.lastTimestamp = getTimestamp();
  }
}

export const normalizeClockFps = (fps: number): number => {
  if (!Number.isFinite(fps) || fps <= 0) {
    return 0;
  }
  // ASM cadence is 59.7275 Hz; normalize legacy "60" callers to the true GB rate.
  if (Math.abs(fps - 60) < 0.001) {
    return GB_FRAME_RATE;
  }
  return fps;
};

export type GameEngineEventQueue = GameEngineEvent[];

const defaultEventQueue: GameEngineEventQueue = [];
let activeEventQueue: GameEngineEventQueue = defaultEventQueue;

const event = {
  Event: EngineEvent,
  createQueue(): GameEngineEventQueue {
    return [];
  },
  getActiveQueue(): GameEngineEventQueue {
    return activeEventQueue;
  },
  setActiveQueue(queue: GameEngineEventQueue | null): void {
    activeEventQueue = queue ?? defaultEventQueue;
  },
  post(eventItem: GameEngineEvent, queue?: GameEngineEventQueue): void {
    const target = queue ?? activeEventQueue;
    target.push(eventItem);
  },
  get(queue?: GameEngineEventQueue, target?: GameEngineEvent[]): GameEngineEvent[] {
    const source = queue ?? activeEventQueue;
    if (arguments.length > 1 && target) {
      target.length = 0;
      for (let index = 0; index < source.length; index += 1) {
        target.push(source[index]!);
      }
      source.length = 0;
      return target;
    }
    const events = source.slice();
    source.length = 0;
    return events;
  },
};

const time = {
  delay(ms: number): void {
    const duration = Math.max(0, ms);
    const target = getTimestamp() + duration;
    while (getTimestamp() < target) {
      // intentionally busy-wait so synchronous loops behave as expected
    }
  },
  Clock,
};

const transform = {
  scale(source: Surface, size: [number, number]): Surface {
    const [width, height] = size;
    const surface = new Surface(width, height);
    const ctx = surface.getContext();
    if (!ctx) {
      throw new Error("Failed to get 2D context for scale");
    }
    if (isHeadlessCanvas(source.canvas)) {
      const srcWidth = source.canvas.width;
      const srcHeight = source.canvas.height;
      const srcImage = source.getImageData();
      const srcData = srcImage.data as Uint8ClampedArray;
      const output = ctx.createImageData(width, height);
      const out = output.data;
      for (let y = 0; y < height; y += 1) {
        const srcY = Math.floor((y / height) * srcHeight);
        for (let x = 0; x < width; x += 1) {
          const srcX = Math.floor((x / width) * srcWidth);
          const srcIndex = (srcY * srcWidth + srcX) * 4;
          const dstIndex = (y * width + x) * 4;
          out[dstIndex] = srcData[srcIndex] ?? 0;
          out[dstIndex + 1] = srcData[srcIndex + 1] ?? 0;
          out[dstIndex + 2] = srcData[srcIndex + 2] ?? 0;
          out[dstIndex + 3] = srcData[srcIndex + 3] ?? 0;
        }
      }
      ctx.putImageData(output, 0, 0);
      return surface;
    }
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(ensureCanvasImageSource(source.canvas), 0, 0, width, height);
    return surface;
  },
  flip(source: Surface, xflip: boolean, yflip: boolean): Surface {
    const width = source.canvas.width;
    const height = source.canvas.height;
    const surface = new Surface(width, height);
    const ctx = surface.getContext();
    if (!ctx) {
      throw new Error("Failed to get 2D context for flip");
    }
    if (isHeadlessCanvas(source.canvas)) {
      const srcImage = source.getImageData();
      const srcData = srcImage.data as Uint8ClampedArray;
      const output = ctx.createImageData(width, height);
      const out = output.data;
      for (let y = 0; y < height; y += 1) {
        const srcY = yflip ? height - 1 - y : y;
        for (let x = 0; x < width; x += 1) {
          const srcX = xflip ? width - 1 - x : x;
          const srcIndex = (srcY * width + srcX) * 4;
          const dstIndex = (y * width + x) * 4;
          out[dstIndex] = srcData[srcIndex] ?? 0;
          out[dstIndex + 1] = srcData[srcIndex + 1] ?? 0;
          out[dstIndex + 2] = srcData[srcIndex + 2] ?? 0;
          out[dstIndex + 3] = srcData[srcIndex + 3] ?? 0;
        }
      }
      ctx.putImageData(output, 0, 0);
      return surface;
    }
    ctx.save();
    ctx.translate(xflip ? width : 0, yflip ? height : 0);
    ctx.scale(xflip ? -1 : 1, yflip ? -1 : 1);
    ctx.drawImage(ensureCanvasImageSource(source.canvas), 0, 0);
    ctx.restore();
    return surface;
  },
};

const draw = {
  rect(
    surface: Surface,
    color: [number, number, number] | [number, number, number, number],
    rect: Rect
  ): void {
    const [r, g, b, a = 255] = color;
    const ctx = surface.getContext();
    if (!ctx) {
      throw new Error("Failed to get 2D context for rect");
    }
    ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a / 255})`;
    ctx.fillRect(rect.x, rect.y, rect.width, rect.height);
  },
  line(
    surface: Surface,
    color: [number, number, number] | [number, number, number, number],
    start: [number, number],
    end: [number, number],
    width = 1
  ): void {
    const [r, g, b, a = 255] = color;
    const ctx = surface.getContext();
    if (!ctx) {
      throw new Error("Failed to get 2D context for line");
    }
    ctx.strokeStyle = `rgba(${r}, ${g}, ${b}, ${a / 255})`;
    ctx.lineWidth = width;
    ctx.beginPath();
    ctx.moveTo(start[0], start[1]);
    ctx.lineTo(end[0], end[1]);
    ctx.stroke();
  },
  circle(
    surface: Surface,
    color: [number, number, number] | [number, number, number, number],
    center: [number, number],
    radius: number,
    width = 0
  ): void {
    const [r, g, b, a = 255] = color;
    const ctx = surface.getContext();
    if (!ctx) {
      throw new Error("Failed to get 2D context for circle");
    }
    ctx.beginPath();
    ctx.arc(center[0], center[1], radius, 0, Math.PI * 2);
    if (width > 0) {
      ctx.strokeStyle = `rgba(${r}, ${g}, ${b}, ${a / 255})`;
      ctx.lineWidth = width;
      ctx.stroke();
    } else {
      ctx.fillStyle = `rgba(${r}, ${g}, ${b}, ${a / 255})`;
      ctx.fill();
    }
  },
};

type DisplayFlipHook = (surface: Surface | null) => void;

interface DisplayState {
  initialized: boolean;
  surface: Surface | null;
  listeners: Set<DisplayFlipHook>;
}

const displayState: DisplayState = {
  initialized: true,
  surface: null,
  listeners: new Set(),
};

const display = {
  flip(): void {
    if (!displayState.initialized) {
      return;
    }
    for (const listener of displayState.listeners) {
      listener(displayState.surface);
    }
  },
  get_init(): boolean {
    return displayState.initialized;
  },
  get_surface(): Surface | null {
    return displayState.surface;
  },
  initialize(surface?: Surface | null): void {
    if (typeof surface === "undefined") {
      displayState.initialized = true;
      return;
    }
    displayState.surface = surface;
    displayState.initialized = surface !== null;
  },
  onFlip(listener: DisplayFlipHook): () => void {
    displayState.listeners.add(listener);
    return () => displayState.listeners.delete(listener);
  },
};

let quitRequested = false;

const requestQuit = (): void => {
  if (quitRequested) {
    return;
  }
  quitRequested = true;
  display.initialize(null);
};

export class GameEngineError extends Error {}

export const gameEngine = {
  SRCALPHA: 1,
  KEYDOWN: "keydown",
  KEYUP: "keyup",
  QUIT: "quit",
  K_UP: "ArrowUp",
  K_DOWN: "ArrowDown",
  K_LEFT: "ArrowLeft",
  K_RIGHT: "ArrowRight",
  K_q: "KeyQ",
  K_e: "KeyE",
  K_RETURN: "Enter",
  K_KP_ENTER: "NumpadEnter",
  K_BACKSPACE: "Backspace",
  Rect: Rect,
  Surface: Surface,
  image: imageLoader,
  event,
  time,
  transform,
  draw,
  display,
  error: GameEngineError,
  quit: requestQuit,
};
