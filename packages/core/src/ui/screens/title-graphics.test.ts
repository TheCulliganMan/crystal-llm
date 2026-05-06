/** @jest-environment jsdom */
import { TitleGraphics } from "./title-graphics";
import fs from "node:fs/promises";
import path from "node:path";
import { PNG } from "pngjs";

const makePaletteText = (): string =>
  Array.from({ length: 64 }, (_, idx) => {
    const value = idx % 32;
    return `RGB ${value}, ${value}, ${value}`;
  }).join("\n");

const make2bppBytes = (length: number): Uint8Array => {
  const bytes = new Uint8Array(length);
  for (let idx = 0; idx < bytes.length; idx += 1) {
    bytes[idx] = idx & 0xff;
  }
  return bytes;
};

const makeSolidTile2bppBytes = (levels: number[]): Uint8Array => {
  const bytes = new Uint8Array(levels.length * 16);
  levels.forEach((level, tileIndex) => {
    for (let row = 0; row < 8; row += 1) {
      const offset = tileIndex * 16 + row * 2;
      bytes[offset] = (level & 1) !== 0 ? 0xff : 0x00;
      bytes[offset + 1] = (level & 2) !== 0 ? 0xff : 0x00;
    }
  });
  return bytes;
};

const expectSolidTileIndex = (tile: number[][], expected: number): void => {
  expect(tile).toHaveLength(8);
  for (const row of tile) {
    expect(row).toEqual(Array(8).fill(expected));
  }
};

const makeLocalTitleAssetFetch = () =>
  jest.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    const assetName = url.split("/").pop();
    if (!assetName) {
      return { ok: false } as Response;
    }
    const bytes = await fs.readFile(path.resolve(process.cwd(), "../../apps/web/assets/gfx/title", assetName));
    return {
      ok: true,
      text: async () => new TextDecoder().decode(bytes),
      arrayBuffer: async () => bytes.buffer.slice(bytes.byteOffset, bytes.byteOffset + bytes.byteLength),
    } as Response;
  });

const pngGrayToTileIndex = (gray: number): number => {
  if (gray === 255) return 0;
  if (gray === 170) return 1;
  if (gray === 85) return 2;
  if (gray === 0) return 3;
  throw new Error(`Unexpected title PNG gray value ${gray}.`);
};

describe("TitleGraphics", () => {
  const originalFetch = global.fetch;
  const makeTextResponse = (body: string, status = 200) =>
    ({
      ok: status >= 200 && status < 300,
      status,
      text: async () => body,
      arrayBuffer: async () => new TextEncoder().encode(body).buffer,
    }) as Response;
  const makeBinaryResponse = (body: Uint8Array, status = 200) =>
    ({
      ok: status >= 200 && status < 300,
      status,
      text: async () => new TextDecoder().decode(body),
      arrayBuffer: async () => body.buffer.slice(body.byteOffset, body.byteOffset + body.byteLength),
    }) as Response;

  afterEach(() => {
    global.fetch = originalFetch;
    jest.restoreAllMocks();
  });

  it("accepts a slightly short logo.2bpp asset without falling back to .lz", async () => {
    const fetchMock = jest.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/assets/gfx/title/title.pal")) {
        return makeTextResponse(makePaletteText(), 200);
      }
      if (url.endsWith("/assets/gfx/title/suicune.2bpp")) {
        return makeBinaryResponse(make2bppBytes(16 * 16 * 16), 200);
      }
      if (url.endsWith("/assets/gfx/title/logo.2bpp")) {
        return makeBinaryResponse(make2bppBytes(2496), 200);
      }
      if (url.endsWith("/assets/gfx/title/crystal.2bpp")) {
        return makeBinaryResponse(make2bppBytes(6 * 10 * 16), 200);
      }
      return makeTextResponse("", 404);
    });
    global.fetch = fetchMock as unknown as typeof fetch;

    const graphics = await TitleGraphics.create();
    const tile = graphics.getTile("logo", 0x80, 0);

    expect(tile).toHaveLength(8);
    expect(tile[0]).toHaveLength(8);
    expect(fetchMock).not.toHaveBeenCalledWith(expect.stringContaining("logo.2bpp.lz"));
  });

  it("orders crystal tiles as 8x16 sprite pairs instead of row-major 8x8 tiles", async () => {
    const crystalLevels = Array(6 * 10).fill(0);
    crystalLevels[0] = 1; // top-left 8x8 tile
    crystalLevels[1] = 2; // top row, second column
    crystalLevels[6] = 3; // bottom half of the leftmost 8x16 sprite

    global.fetch = jest.fn(async (input: RequestInfo | URL) => {
      const url = String(input);
      if (url.endsWith("/assets/gfx/title/title.pal")) {
        return makeTextResponse(makePaletteText(), 200);
      }
      if (url.endsWith("/assets/gfx/title/suicune.2bpp")) {
        return makeBinaryResponse(make2bppBytes(16 * 16 * 16), 200);
      }
      if (url.endsWith("/assets/gfx/title/logo.2bpp")) {
        return makeBinaryResponse(make2bppBytes(20 * 8 * 16), 200);
      }
      if (url.endsWith("/assets/gfx/title/crystal.2bpp")) {
        return makeBinaryResponse(makeSolidTile2bppBytes(crystalLevels), 200);
      }
      return makeTextResponse("", 404);
    }) as unknown as typeof fetch;

    const graphics = await TitleGraphics.create();

    expectSolidTileIndex(graphics.getTileIndices("crystal", 0), 1);
    expectSolidTileIndex(graphics.getTileIndices("crystal", 1), 3);
    expectSolidTileIndex(graphics.getTileIndices("crystal", 2), 2);
  });

  it("matches the vendor Crystal PNG when assembled through the title-screen OAM tilemap", async () => {
    global.fetch = makeLocalTitleAssetFetch() as unknown as typeof fetch;
    const graphics = await TitleGraphics.create();
    const pngBytes = await fs.readFile(path.resolve(process.cwd(), "../../vendor/pokecrystal/gfx/title/crystal.png"));
    const png = PNG.sync.read(pngBytes);
    let tileId = 0;

    expect(png.width).toBe(48);
    expect(png.height).toBe(80);

    for (let spriteRow = 0; spriteRow < 5; spriteRow += 1) {
      for (let spriteCol = 0; spriteCol < 6; spriteCol += 1) {
        for (let half = 0; half < 2; half += 1) {
          const tile = graphics.getTileIndices("crystal", tileId + half);
          for (let row = 0; row < 8; row += 1) {
            for (let col = 0; col < 8; col += 1) {
              const pixelX = spriteCol * 8 + col;
              const pixelY = spriteRow * 16 + half * 8 + row;
              const pixelOffset = (pixelY * png.width + pixelX) * 4;
              expect(tile[row][col]).toBe(pngGrayToTileIndex(png.data[pixelOffset]));
            }
          }
        }
        tileId += 2;
      }
    }
  });
});
