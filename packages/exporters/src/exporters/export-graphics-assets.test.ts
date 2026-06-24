import fs from "fs";
import os from "os";
import path from "path";
import { exportGraphicsAssets } from "./export-graphics-assets";

const { PNG } = require("pngjs") as any;

let mockDisassemblyRoot = "";
let mockDataDir = "";

jest.mock("@pokecrystal/core/core/paths", () => ({
  getDisassemblyRoot: () => mockDisassemblyRoot,
  getAssetsRoot: () => require("path").dirname(mockDataDir),
}));

const writePng = (filePath: string, width: number, height: number, grayForPixel: (x: number, y: number) => number): void => {
  const png = new PNG({ width, height });
  for (let y = 0; y < height; y += 1) {
    for (let x = 0; x < width; x += 1) {
      const offset = (y * width + x) * 4;
      const gray = grayForPixel(x, y);
      png.data[offset] = gray;
      png.data[offset + 1] = gray;
      png.data[offset + 2] = gray;
      png.data[offset + 3] = 255;
    }
  }
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, PNG.sync.write(png));
};

describe("exportGraphicsAssets", () => {
  let tempDir: string;

  beforeEach(() => {
    tempDir = fs.mkdtempSync(path.join(os.tmpdir(), "pokecrystal-export-gfx-"));
    mockDisassemblyRoot = path.join(tempDir, "vendor");
    mockDataDir = path.join(tempDir, "assets", "data");
  });

  afterEach(() => {
    fs.rmSync(tempDir, { recursive: true, force: true });
  });

  it("copies graphics, filters .DS_Store, and converts PNG sources to raw bpp files", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "battle"), { recursive: true });
    fs.writeFileSync(path.join(sourceGfx, ".DS_Store"), "ignore");
    fs.writeFileSync(path.join(sourceGfx, "battle", "kept.txt"), "copied");
    writePng(path.join(sourceGfx, "battle", "solid.png"), 8, 8, () => 255);

    exportGraphicsAssets();

    const targetGfx = path.join(tempDir, "assets", "gfx");
    expect(fs.existsSync(path.join(targetGfx, ".DS_Store"))).toBe(false);
    expect(fs.readFileSync(path.join(targetGfx, "battle", "kept.txt"), "utf8")).toBe("copied");
    expect(fs.readFileSync(path.join(targetGfx, "battle", "solid.1bpp"))).toEqual(
      Buffer.from(Array(8).fill(0x00))
    );
    expect(fs.readFileSync(path.join(targetGfx, "battle", "solid.2bpp"))).toEqual(
      Buffer.from(Array(16).fill(0x00))
    );
  });

  it("encodes battle PNG sources in RGBDS color order", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "battle"), { recursive: true });
    writePng(path.join(sourceGfx, "battle", "levels.png"), 8, 8, (x) => [255, 170, 85, 0, 255, 170, 85, 0][x]);

    exportGraphicsAssets();

    const encoded2bpp = fs.readFileSync(path.join(tempDir, "assets", "gfx", "battle", "levels.2bpp"));
    expect([...encoded2bpp.slice(0, 2)]).toEqual([0x55, 0x33]);

    const encoded1bpp = fs.readFileSync(path.join(tempDir, "assets", "gfx", "battle", "levels.1bpp"));
    expect(encoded1bpp[0]).toBe(0x33);
  });

  it("exports player backpic PNG sources as 2bpp battle assets", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "player"), { recursive: true });
    writePng(path.join(sourceGfx, "player", "chris_back.png"), 8, 8, (x) => (x < 4 ? 255 : 0));

    exportGraphicsAssets();

    const encoded = fs.readFileSync(path.join(tempDir, "assets", "gfx", "player", "chris_back.2bpp"));
    expect([...encoded.slice(0, 2)]).toEqual([0x0f, 0x0f]);
  });

  it("exports battle animation PNG sources as 2bpp graphics", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "battle_anims"), { recursive: true });
    writePng(path.join(sourceGfx, "battle_anims", "pokeball.png"), 8, 8, (x) => [255, 170, 85, 0, 255, 170, 85, 0][x]);

    exportGraphicsAssets();

    const encoded = fs.readFileSync(path.join(tempDir, "assets", "gfx", "battle_anims", "pokeball.2bpp"));
    expect([...encoded.slice(0, 2)]).toEqual([0x55, 0x33]);
  });

  it("exports lava tileset animation PNG sources as 2bpp graphics", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "tilesets", "lava"), { recursive: true });
    writePng(path.join(sourceGfx, "tilesets", "lava", "1.png"), 8, 8, (x) => [255, 170, 85, 0][x % 4]);

    exportGraphicsAssets();

    const encoded = fs.readFileSync(path.join(tempDir, "assets", "gfx", "tilesets", "lava", "1.2bpp"));
    expect([...encoded.slice(0, 2)]).toEqual([0x55, 0x33]);
  });

  it("exports forest tree tileset animation PNG sources as 2bpp graphics", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "tilesets", "forest-tree"), { recursive: true });
    writePng(path.join(sourceGfx, "tilesets", "forest-tree", "1.png"), 8, 8, (x) => [255, 170, 85, 0][x % 4]);

    exportGraphicsAssets();

    const encoded = fs.readFileSync(path.join(tempDir, "assets", "gfx", "tilesets", "forest-tree", "1.2bpp"));
    expect([...encoded.slice(0, 2)]).toEqual([0x55, 0x33]);
  });

  it("exports pack menu PNG sources as 2bpp graphics", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "pack"), { recursive: true });
    writePng(path.join(sourceGfx, "pack", "pack_menu.png"), 8, 8, (x) => [255, 170, 85, 0, 255, 170, 85, 0][x]);

    exportGraphicsAssets();

    const encoded = fs.readFileSync(path.join(tempDir, "assets", "gfx", "pack", "pack_menu.2bpp"));
    expect([...encoded.slice(0, 2)]).toEqual([0x55, 0x33]);
  });

  it("exports party menu icon and stat tile PNG sources as asm-addressable 2bpp graphics", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "icons"), { recursive: true });
    fs.mkdirSync(path.join(sourceGfx, "stats"), { recursive: true });
    writePng(path.join(sourceGfx, "icons", "monster.png"), 16, 32, (x) => [255, 170, 85, 0][x % 4]);
    writePng(path.join(sourceGfx, "stats", "item.png"), 8, 8, (x) => [255, 170, 85, 0][x % 4]);
    writePng(path.join(sourceGfx, "stats", "stats_tiles.png"), 16, 8, (x) => [255, 170, 85, 0][x % 4]);

    exportGraphicsAssets();

    const targetGfx = path.join(tempDir, "assets", "gfx");
    const icon = fs.readFileSync(path.join(targetGfx, "icons", "monster.2bpp"));
    expect(icon.length).toBe(8 * 16);
    expect([...icon.slice(0, 2)]).toEqual([0x55, 0x33]);

    const item = fs.readFileSync(path.join(targetGfx, "stats", "item.2bpp"));
    expect(item.length).toBe(16);
    expect([...item.slice(0, 2)]).toEqual([0x55, 0x33]);

    const statsTiles = fs.readFileSync(path.join(targetGfx, "stats", "stats_tiles.2bpp"));
    expect(statsTiles.length).toBe(2 * 16);
  });

  it("exports trainer PNG sources as 2bpp graphics and indexed gbcpal palettes", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "trainers"), { recursive: true });
    writePng(path.join(sourceGfx, "trainers", "cal.png"), 8, 8, (x) => [255, 170, 85, 0, 255, 170, 85, 0][x]);

    exportGraphicsAssets();

    const encoded = fs.readFileSync(path.join(tempDir, "assets", "gfx", "trainers", "cal.2bpp"));
    expect([...encoded.slice(0, 2)]).toEqual([0x55, 0x33]);

    const palette = fs.readFileSync(path.join(tempDir, "assets", "gfx", "trainers", "cal.gbcpal"));
    expect(palette.length).toBe(8);
    expect(palette.readUInt16LE(0)).toBe(0x7fff);
    expect(palette.readUInt16LE(6)).toBe(0x0000);
  });

  it("exports pokemon normal gbcpal palettes from front and back PNG sources", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    const speciesDir = path.join(sourceGfx, "pokemon", "croconaw");
    fs.mkdirSync(speciesDir, { recursive: true });
    writePng(path.join(speciesDir, "front.png"), 8, 8, (x) => [255, 96, 32, 0][x % 4]);
    writePng(path.join(speciesDir, "back.png"), 8, 8, (x) => [255, 96, 32, 0][x % 4]);

    exportGraphicsAssets();

    const targetDir = path.join(tempDir, "assets", "gfx", "pokemon", "croconaw");
    expect(fs.existsSync(path.join(targetDir, "front.2bpp"))).toBe(true);
    expect(fs.existsSync(path.join(targetDir, "back.2bpp"))).toBe(true);

    const palette = fs.readFileSync(path.join(targetDir, "normal.gbcpal"));
    expect(palette.length).toBe(8);
    expect(palette.readUInt16LE(0)).toBe(0x7fff);
    expect(palette.readUInt16LE(2)).toBe(0x318c);
    expect(palette.readUInt16LE(4)).toBe(0x1084);
    expect(palette.readUInt16LE(6)).toBe(0x0000);
  });

  it("honors upstream reversed pokemon normal palette ordering", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    const speciesDir = path.join(sourceGfx, "pokemon", "scyther");
    fs.mkdirSync(speciesDir, { recursive: true });
    writePng(path.join(speciesDir, "front.png"), 8, 8, (x) => [255, 96, 32, 0][x % 4]);
    writePng(path.join(speciesDir, "back.png"), 8, 8, (x) => [255, 96, 32, 0][x % 4]);

    exportGraphicsAssets();

    const palette = fs.readFileSync(path.join(tempDir, "assets", "gfx", "pokemon", "scyther", "normal.gbcpal"));
    expect(palette.readUInt16LE(2)).toBe(0x1084);
    expect(palette.readUInt16LE(4)).toBe(0x318c);
  });

  it("encodes font 2bpp PNGs with white backgrounds as transparent level zero", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "font"), { recursive: true });
    writePng(path.join(sourceGfx, "font", "space.png"), 8, 8, (x) => [0, 85, 170, 255, 0, 85, 170, 255][x]);

    exportGraphicsAssets();

    const encoded = fs.readFileSync(path.join(tempDir, "assets", "gfx", "font", "space.2bpp"));
    expect([...encoded.slice(0, 2)]).toEqual([0xaa, 0xcc]);
  });

  it("keeps transparent font pixels at palette level zero", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "font"), { recursive: true });
    const png = new PNG({ width: 8, height: 8 });
    for (let y = 0; y < 8; y += 1) {
      for (let x = 0; x < 8; x += 1) {
        const offset = (y * 8 + x) * 4;
        png.data[offset] = 0;
        png.data[offset + 1] = 0;
        png.data[offset + 2] = 0;
        png.data[offset + 3] = x === 0 && y === 7 ? 255 : 0;
      }
    }
    fs.writeFileSync(path.join(sourceGfx, "font", "font_extra.png"), PNG.sync.write(png));

    exportGraphicsAssets();

    const encoded = fs.readFileSync(path.join(tempDir, "assets", "gfx", "font", "font_extra.2bpp"));
    expect([...encoded.slice(0, 14)]).toEqual(Array(14).fill(0));
    expect([...encoded.slice(14, 16)]).toEqual([0x80, 0x80]);
  });

  it("rejects PNG sources that are not aligned to 8x8 tiles", () => {
    const sourceGfx = path.join(mockDisassemblyRoot, "gfx");
    fs.mkdirSync(path.join(sourceGfx, "battle"), { recursive: true });
    writePng(path.join(sourceGfx, "battle", "bad.png"), 7, 8, () => 0);

    expect(() => exportGraphicsAssets()).toThrow("must align to 8x8 tiles");
  });

  it("throws when the graphics source directory is missing", () => {
    expect(() => exportGraphicsAssets()).toThrow("Missing graphics source directory");
  });
});
