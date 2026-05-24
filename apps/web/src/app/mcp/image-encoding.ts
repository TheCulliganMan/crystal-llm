import { deflateSync } from "zlib";
import type { Surface } from "@pokecrystal/core/ui/surface";

type PngOptions = {
  scale?: number;
};

const PNG_SIGNATURE = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);

let crcTable: Uint32Array | null = null;

const getCrcTable = (): Uint32Array => {
  if (crcTable) {
    return crcTable;
  }
  const table = new Uint32Array(256);
  for (let i = 0; i < 256; i += 1) {
    let c = i;
    for (let k = 0; k < 8; k += 1) {
      c = (c & 1) ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    }
    table[i] = c >>> 0;
  }
  crcTable = table;
  return table;
};

const crc32 = (data: Uint8Array): number => {
  const table = getCrcTable();
  let crc = 0xffffffff;
  for (let i = 0; i < data.length; i += 1) {
    const value = (crc ^ data[i]) & 0xff;
    crc = table[value] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
};

const buildChunk = (type: string, data: Uint8Array): Buffer => {
  const typeBytes = Buffer.from(type, "ascii");
  if (typeBytes.length !== 4) {
    throw new Error(`PNG chunk type must be 4 bytes, got ${type}.`);
  }
  const chunk = Buffer.alloc(8 + data.length + 4);
  chunk.writeUInt32BE(data.length, 0);
  typeBytes.copy(chunk, 4);
  Buffer.from(data).copy(chunk, 8);
  const crcInput = new Uint8Array(typeBytes.length + data.length);
  crcInput.set(typeBytes, 0);
  crcInput.set(data, typeBytes.length);
  chunk.writeUInt32BE(crc32(crcInput), 8 + data.length);
  return chunk;
};

const normalizeScale = (scale: number | undefined): number => {
  if (scale === undefined) {
    return 1;
  }
  if (!Number.isFinite(scale)) {
    throw new Error("Scale must be a finite number.");
  }
  const normalized = Math.floor(scale);
  if (normalized < 1 || normalized > 8) {
    throw new Error("Scale must be an integer between 1 and 8.");
  }
  return normalized;
};

const scaleRgba = (
  data: Uint8Array,
  width: number,
  height: number,
  scale: number
): { data: Uint8Array; width: number; height: number } => {
  if (scale === 1) {
    return { data, width, height };
  }
  const scaledWidth = width * scale;
  const scaledHeight = height * scale;
  const scaled = new Uint8Array(scaledWidth * scaledHeight * 4);
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const srcIndex = (y * width + x) * 4;
      const r = data[srcIndex];
      const g = data[srcIndex + 1];
      const b = data[srcIndex + 2];
      const a = data[srcIndex + 3];
      for (let dy = 0; dy < scale; dy += 1) {
        const rowStart = ((y * scale + dy) * scaledWidth + x * scale) * 4;
        for (let dx = 0; dx < scale; dx += 1) {
          const destIndex = rowStart + dx * 4;
          scaled[destIndex] = r;
          scaled[destIndex + 1] = g;
          scaled[destIndex + 2] = b;
          scaled[destIndex + 3] = a;
        }
      }
    }
  }
  return { data: scaled, width: scaledWidth, height: scaledHeight };
};

const encodeRgbaToPng = (
  rgbaData: Uint8Array,
  width: number,
  height: number,
  scale: number
): { data: string; width: number; height: number } => {
  const scaled = scaleRgba(rgbaData, width, height, scale);
  const bytesPerRow = scaled.width * 4;
  const scanline = new Uint8Array((bytesPerRow + 1) * scaled.height);
  for (let y = 0; y < scaled.height; y += 1) {
    const srcStart = y * bytesPerRow;
    const destStart = y * (bytesPerRow + 1);
    scanline[destStart] = 0;
    scanline.set(scaled.data.subarray(srcStart, srcStart + bytesPerRow), destStart + 1);
  }
  const compressed = deflateSync(scanline);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(scaled.width, 0);
  ihdr.writeUInt32BE(scaled.height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;

  const chunks = [
    Buffer.from(PNG_SIGNATURE),
    buildChunk("IHDR", ihdr),
    buildChunk("IDAT", compressed),
    buildChunk("IEND", new Uint8Array()),
  ];
  const png = Buffer.concat(chunks);
  return {
    data: png.toString("base64"),
    width: scaled.width,
    height: scaled.height,
  };
};

export const encodeSurfaceToPng = (
  surface: Surface,
  options: PngOptions = {}
): { data: string; width: number; height: number } => {
  const width = surface.get_width();
  const height = surface.get_height();
  if (width <= 0 || height <= 0) {
    throw new Error(`Surface size must be positive, got ${width}x${height}.`);
  }
  const scale = normalizeScale(options.scale);
  const expectedLength = width * height * 4;
  const image = surface.getImageData();
  const rgba = new Uint8Array(expectedLength);
  const src = image?.data as ArrayLike<number> | undefined;
  if (src) {
    const limit = Math.min(expectedLength, src.length);
    for (let i = 0; i < limit; i += 1) {
      rgba[i] = src[i] ?? 0;
    }
  }
  return encodeRgbaToPng(rgba, width, height, scale);
};
