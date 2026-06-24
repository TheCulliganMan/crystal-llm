import fs from "fs";
import path from "path";
import { getDisassemblyRoot } from "@pokecrystal/core/core/paths";
import { ensureDir, getTypeScriptDataDir } from "./asm-utils";

const { PNG } = require("pngjs") as any;
const TILE_SIZE = 8;

type Rgb = [number, number, number];

const WHITE_5: Rgb = [31, 31, 31];
const BLACK_5: Rgb = [0, 0, 0];
const REVERSED_POKEMON_PALETTES = new Set([
  "spearow",
  "fearow",
  "farfetch_d",
  "hitmonlee",
  "scyther",
  "jynx",
  "porygon",
  "porygon2",
]);

const readPngPalette = (filePath: string): Rgb[] => {
  const data = fs.readFileSync(filePath);
  if (data.length < 33 || data.toString("ascii", 1, 4) !== "PNG") {
    return [];
  }
  const colours: Rgb[] = [];
  let offset = 8;
  while (offset + 12 <= data.length) {
    const length = data.readUInt32BE(offset);
    const type = data.toString("ascii", offset + 4, offset + 8);
    const start = offset + 8;
    const end = start + length;
    if (end + 4 > data.length) {
      break;
    }
    if (type === "PLTE") {
      for (let index = start; index + 2 < end; index += 3) {
        colours.push([data[index] ?? 0, data[index + 1] ?? 0, data[index + 2] ?? 0]);
      }
      return colours;
    }
    offset = end + 4;
  }
  return colours;
};

const rgb8ToRgb5 = ([r, g, b]: Rgb): Rgb => [
  Math.max(0, Math.min(31, Math.round((r / 255) * 31))),
  Math.max(0, Math.min(31, Math.round((g / 255) * 31))),
  Math.max(0, Math.min(31, Math.round((b / 255) * 31))),
];

const rgb8ToGbcWord = ([r, g, b]: Rgb): number => {
  const [r5, g5, b5] = rgb8ToRgb5([r, g, b]);
  return r5 | (g5 << 5) | (b5 << 10);
};

const rgb5ToGbcWord = ([r, g, b]: Rgb): number => r | (g << 5) | (b << 10);

const readGbcpal = (filePath: string): Rgb[] => {
  const data = fs.readFileSync(filePath);
  if (data.length === 0 || data.length % 2 !== 0) {
    throw new Error(`Invalid GBC palette ${filePath}: expected a non-empty even number of bytes.`);
  }
  const colours: Rgb[] = [];
  for (let offset = 0; offset < data.length; offset += 2) {
    const value = data.readUInt16LE(offset);
    colours.push([value & 0x1f, (value >> 5) & 0x1f, (value >> 10) & 0x1f]);
  }
  return colours;
};

const writeGbcpal = (targetPath: string, colours: Rgb[], rgb5 = false): void => {
  const bytes = Buffer.alloc(colours.length * 2);
  colours.forEach((colour, index) => {
    bytes.writeUInt16LE(rgb5 ? rgb5ToGbcWord(colour) : rgb8ToGbcWord(colour), index * 2);
  });
  fs.writeFileSync(targetPath, bytes);
};

const writeGbcpalFromPng = (pngPath: string): void => {
  let palette = readPngPalette(pngPath);
  if (palette.length < 4) {
    const image = PNG.sync.read(fs.readFileSync(pngPath));
    const seen = new Set<string>();
    palette = [];
    for (let offset = 0; offset < image.data.length; offset += 4) {
      if ((image.data[offset + 3] ?? 255) === 0) {
        continue;
      }
      const colour: Rgb = [image.data[offset] ?? 0, image.data[offset + 1] ?? 0, image.data[offset + 2] ?? 0];
      const key = colour.join(",");
      if (!seen.has(key)) {
        seen.add(key);
        palette.push(colour);
      }
    }
  }
  if (palette.length < 4) {
    throw new Error(`Trainer PNG ${pngPath} must contain at least 4 indexed palette colours.`);
  }
  const targetPath = pngPath.replace(/\.png$/i, ".gbcpal");
  writeGbcpal(targetPath, palette.slice(0, 4));
};

const sameRgb = (left: Rgb, right: Rgb): boolean =>
  left[0] === right[0] && left[1] === right[1] && left[2] === right[2];

const luminance = ([r, g, b]: Rgb): number => 0.299 * r + 0.587 * g + 0.114 * b;

const writePokemonNormalGbcpal = (speciesDir: string, reverse = false): void => {
  const sourcePalettes = ["front.gbcpal", "back.gbcpal"]
    .map((name) => path.join(speciesDir, name))
    .filter((palettePath) => fs.existsSync(palettePath));
  if (!sourcePalettes.length) {
    return;
  }
  const middleColours = sourcePalettes
    .flatMap((palettePath) => readGbcpal(palettePath))
    .filter((colour) => !sameRgb(colour, WHITE_5) && !sameRgb(colour, BLACK_5))
    .sort((left, right) => {
      const comparison = luminance(right) - luminance(left);
      return reverse ? -comparison : comparison;
    })
    .filter((colour, index, colours) => index === 0 || !sameRgb(colour, colours[index - 1] ?? BLACK_5));

  if (middleColours.length > 2) {
    throw new Error(`${path.join(speciesDir, "normal.gbcpal")}: more than 2 colors besides black and white (${middleColours.length}).`);
  }

  const palette: Rgb[] = [
    WHITE_5,
    middleColours[0] ?? WHITE_5,
    middleColours[1] ?? middleColours[0] ?? BLACK_5,
    BLACK_5,
  ];
  writeGbcpal(path.join(speciesDir, "normal.gbcpal"), palette, true);
};

const readPngLevels = (
  filePath: string,
  maxLevel: 1 | 3
): number[][] => {
  const image = PNG.sync.read(fs.readFileSync(filePath));
  if (image.width % TILE_SIZE !== 0 || image.height % TILE_SIZE !== 0) {
    throw new Error(`PNG tileset ${filePath} must align to ${TILE_SIZE}x${TILE_SIZE} tiles.`);
  }
  const tiles: number[][] = [];
  const tilesWide = image.width / TILE_SIZE;
  const tilesHigh = image.height / TILE_SIZE;
  for (let tileY = 0; tileY < tilesHigh; tileY += 1) {
    for (let tileX = 0; tileX < tilesWide; tileX += 1) {
      const levels: number[] = [];
      for (let y = 0; y < TILE_SIZE; y += 1) {
        for (let x = 0; x < TILE_SIZE; x += 1) {
          const sourceX = tileX * TILE_SIZE + x;
          const sourceY = tileY * TILE_SIZE + y;
          const offset = (sourceY * image.width + sourceX) * 4;
          const r = image.data[offset] ?? 0;
          const g = image.data[offset + 1] ?? 0;
          const b = image.data[offset + 2] ?? 0;
          const a = image.data[offset + 3] ?? 255;
          if (a === 0) {
            levels.push(0);
            continue;
          }
          const gray = Math.round((r + g + b) / 3);
          levels.push(Math.round(((255 - gray) / 255) * maxLevel));
        }
      }
      tiles.push(levels);
    }
  }
  return tiles;
};

const encode1bpp = (levelsByTile: number[][]): Buffer => {
  const bytes: number[] = [];
  for (const levels of levelsByTile) {
    for (let y = 0; y < TILE_SIZE; y += 1) {
      let value = 0;
      for (let x = 0; x < TILE_SIZE; x += 1) {
        if ((levels[y * TILE_SIZE + x] ?? 0) > 0) {
          value |= 1 << (7 - x);
        }
      }
      bytes.push(value);
    }
  }
  return Buffer.from(bytes);
};

const encode2bpp = (levelsByTile: number[][]): Buffer => {
  const bytes: number[] = [];
  for (const levels of levelsByTile) {
    for (let y = 0; y < TILE_SIZE; y += 1) {
      let lo = 0;
      let hi = 0;
      for (let x = 0; x < TILE_SIZE; x += 1) {
        const level = levels[y * TILE_SIZE + x] ?? 0;
        const mask = 1 << (7 - x);
        if (level & 1) {
          lo |= mask;
        }
        if (level & 2) {
          hi |= mask;
        }
      }
      bytes.push(lo, hi);
    }
  }
  return Buffer.from(bytes);
};

const writeConvertedPng = (
  pngPath: string,
  extension: ".1bpp" | ".2bpp"
): void => {
  const targetPath = pngPath.replace(/\.png$/i, extension);
  const maxLevel = extension === ".1bpp" ? 1 : 3;
  const levels = readPngLevels(pngPath, maxLevel);
  fs.writeFileSync(targetPath, extension === ".1bpp" ? encode1bpp(levels) : encode2bpp(levels));
};

const convertPngsInDir = (
  dirPath: string,
  extension: ".1bpp" | ".2bpp"
): void => {
  if (!fs.existsSync(dirPath)) {
    return;
  }
  for (const entry of fs.readdirSync(dirPath, { withFileTypes: true })) {
    if (entry.isFile() && entry.name.endsWith(".png")) {
      writeConvertedPng(path.join(dirPath, entry.name), extension);
    }
  }
};

const convertTrainerPngs = (dirPath: string): void => {
  if (!fs.existsSync(dirPath)) {
    return;
  }
  for (const entry of fs.readdirSync(dirPath, { withFileTypes: true })) {
    if (entry.isFile() && entry.name.endsWith(".png")) {
      const pngPath = path.join(dirPath, entry.name);
      writeConvertedPng(pngPath, ".2bpp");
      writeGbcpalFromPng(pngPath);
    }
  }
};

const convertPokemonPngs = (pokemonDir: string): void => {
  if (!fs.existsSync(pokemonDir)) {
    return;
  }
  const unownPalettes: string[] = [];
  for (const entry of fs.readdirSync(pokemonDir, { withFileTypes: true })) {
    if (!entry.isDirectory()) {
      continue;
    }
    const speciesDir = path.join(pokemonDir, entry.name);
    for (const imageName of ["front.png", "back.png"]) {
      const pngPath = path.join(speciesDir, imageName);
      if (!fs.existsSync(pngPath)) {
        continue;
      }
      writeConvertedPng(pngPath, ".2bpp");
      writeGbcpalFromPng(pngPath);
      if (entry.name.startsWith("unown_")) {
        unownPalettes.push(pngPath.replace(/\.png$/i, ".gbcpal"));
      }
    }
    if (!entry.name.startsWith("unown_")) {
      writePokemonNormalGbcpal(speciesDir, REVERSED_POKEMON_PALETTES.has(entry.name));
    }
  }
  const sharedUnownDir = path.join(pokemonDir, "unown");
  if (unownPalettes.length && fs.existsSync(sharedUnownDir)) {
    writePokemonNormalGbcpalFromPalettes(sharedUnownDir, unownPalettes);
  }
};

const writePokemonNormalGbcpalFromPalettes = (
  targetDir: string,
  sourcePalettes: string[],
): void => {
  const middleColours = sourcePalettes
    .flatMap((palettePath) => readGbcpal(palettePath))
    .filter((colour) => !sameRgb(colour, WHITE_5) && !sameRgb(colour, BLACK_5))
    .sort((left, right) => luminance(right) - luminance(left))
    .filter((colour, index, colours) => index === 0 || !sameRgb(colour, colours[index - 1] ?? BLACK_5));
  if (middleColours.length > 2) {
    throw new Error(`${path.join(targetDir, "normal.gbcpal")}: more than 2 colors besides black and white (${middleColours.length}).`);
  }
  writeGbcpal(path.join(targetDir, "normal.gbcpal"), [
    WHITE_5,
    middleColours[0] ?? WHITE_5,
    middleColours[1] ?? middleColours[0] ?? BLACK_5,
    BLACK_5,
  ], true);
};

const syncGeneratedRawGraphics = (target: string): void => {
  for (const relativeDir of [
    "battle",
    "battle_anims",
    "mobile",
    "title",
    "trainer_card",
    "unown_puzzle",
    path.join("tilesets", "flower"),
    path.join("tilesets", "forest-tree"),
    path.join("tilesets", "fountain"),
    path.join("tilesets", "lava"),
    path.join("tilesets", "tower-pillar"),
    path.join("tilesets", "water"),
    path.join("tilesets", "whirlpool"),
    "overworld",
    "pack",
    "player",
    "icons",
    "stats",
  ]) {
    convertPngsInDir(path.join(target, relativeDir), ".2bpp");
  }
  convertTrainerPngs(path.join(target, "trainers"));
  convertPokemonPngs(path.join(target, "pokemon"));
  convertPngsInDir(path.join(target, "battle"), ".1bpp");
  convertPngsInDir(path.join(target, "frames"), ".1bpp");
  const fontDir = path.join(target, "font");
  convertPngsInDir(fontDir, ".2bpp");
  for (const [stem, extension] of [
    ["font", ".1bpp"],
  ] as const) {
    const pngPath = path.join(fontDir, `${stem}.png`);
    if (fs.existsSync(pngPath)) {
      writeConvertedPng(pngPath, extension);
    }
  }
};

export function exportGraphicsAssets(): void {
  const source = path.join(getDisassemblyRoot(), "gfx");
  const target = path.join(path.dirname(getTypeScriptDataDir()), "gfx");

  if (!fs.existsSync(source)) {
    throw new Error(`Missing graphics source directory: ${source}`);
  }

  ensureDir(target);
  fs.cpSync(source, target, {
    recursive: true,
    force: true,
    dereference: false,
    filter: (sourcePath) => path.basename(sourcePath) !== ".DS_Store",
  });
  syncGeneratedRawGraphics(target);
}
