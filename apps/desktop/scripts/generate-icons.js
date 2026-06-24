"use strict";

const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");
const sharp = require("sharp");
const { DESKTOP_ASSETS_DIR, ROOT_DIR } = require("./launch-helpers");

const SPRITE_PNG = path.join(ROOT_DIR, "apps/web/assets/gfx/pokemon/krabby/front.png");
const SOURCE_SVG = path.join(ROOT_DIR, "apps/desktop/assets/icon-source.svg");
const TARGET_PNG = path.join(DESKTOP_ASSETS_DIR, "icon.png");
const TARGET_ICO = path.join(DESKTOP_ASSETS_DIR, "icon.ico");
const TARGET_ICNS = path.join(DESKTOP_ASSETS_DIR, "icon.icns");

const PNG_SIZE = 1024;
const ICONSET_SIZES = [16, 32, 128, 256, 512];
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];

const ensureSourceIcon = (iconPath) => {
  if (!fs.existsSync(iconPath)) {
    throw new Error(`Missing desktop icon source: ${iconPath}`);
  }
};

const runTool = (command, args) => {
  const result = spawnSync(command, args, { stdio: "inherit" });
  if (result.status !== 0) {
    throw new Error(`Command failed (${result.status ?? 1}): ${command} ${args.join(" ")}`);
  }
};

const renderFallbackPng = async (size) =>
  sharp(SOURCE_SVG, { density: 288 }).resize(size, size).png().toBuffer();

const renderTransparentBackground = async (size) =>
  sharp({
    create: {
      width: size,
      height: size,
      channels: 4,
      background: { r: 0, g: 0, b: 0, alpha: 0 },
    },
  }).png().toBuffer();

const removeEdgeBackground = async (input, width, height) => {
  const { data } = await sharp(input).ensureAlpha().raw().toBuffer({ resolveWithObject: true });
  const output = Buffer.from(data);
  const visited = new Uint8Array(width * height);
  const queue = [];
  const background = [output[0], output[1], output[2]];
  const colorTolerance = 8;

  const matchesBackground = (index) => {
    const offset = index * 4;
    return (
      Math.abs(output[offset] - background[0]) <= colorTolerance &&
      Math.abs(output[offset + 1] - background[1]) <= colorTolerance &&
      Math.abs(output[offset + 2] - background[2]) <= colorTolerance
    );
  };

  const enqueue = (x, y) => {
    if (x < 0 || y < 0 || x >= width || y >= height) {
      return;
    }
    const index = y * width + x;
    if (visited[index] || !matchesBackground(index)) {
      return;
    }
    visited[index] = 1;
    queue.push(index);
  };

  for (let x = 0; x < width; x += 1) {
    enqueue(x, 0);
    enqueue(x, height - 1);
  }
  for (let y = 0; y < height; y += 1) {
    enqueue(0, y);
    enqueue(width - 1, y);
  }

  while (queue.length > 0) {
    const index = queue.shift();
    const x = index % width;
    const y = Math.floor(index / width);
    output[index * 4 + 3] = 0;
    enqueue(x + 1, y);
    enqueue(x - 1, y);
    enqueue(x, y + 1);
    enqueue(x, y - 1);
  }

  return sharp(output, { raw: { width, height, channels: 4 } }).png().toBuffer();
};

const renderSpriteIcon = async (size) => {
  const spriteSize = 40;
  const spritePadding = Math.round(size * 0.18);
  const spriteRenderSize = size - spritePadding * 2;
  const background = await renderTransparentBackground(size);
  const spriteSource = await sharp(SPRITE_PNG)
    .extract({ left: 0, top: 0, width: spriteSize, height: spriteSize })
    .png()
    .toBuffer();
  const sprite = await sharp(await removeEdgeBackground(spriteSource, spriteSize, spriteSize))
    .resize(spriteRenderSize, spriteRenderSize, { kernel: sharp.kernel.nearest })
    .png()
    .toBuffer();

  return sharp(background)
    .composite([{ input: sprite, top: spritePadding, left: spritePadding }])
    .png()
    .toBuffer();
};

const renderSourcePng = async (size) => {
  if (fs.existsSync(SPRITE_PNG)) {
    return renderSpriteIcon(size);
  }
  return renderFallbackPng(size);
};

const writeIco = async () => {
  const images = await Promise.all(
    ICO_SIZES.map(async (size) => ({
      size,
      data: await renderSourcePng(size),
    })),
  );

  const header = Buffer.alloc(6 + images.length * 16);
  header.writeUInt16LE(0, 0);
  header.writeUInt16LE(1, 2);
  header.writeUInt16LE(images.length, 4);

  let offset = header.length;
  for (const [index, image] of images.entries()) {
    const entryOffset = 6 + index * 16;
    header.writeUInt8(image.size === 256 ? 0 : image.size, entryOffset);
    header.writeUInt8(image.size === 256 ? 0 : image.size, entryOffset + 1);
    header.writeUInt8(0, entryOffset + 2);
    header.writeUInt8(0, entryOffset + 3);
    header.writeUInt16LE(1, entryOffset + 4);
    header.writeUInt16LE(32, entryOffset + 6);
    header.writeUInt32LE(image.data.length, entryOffset + 8);
    header.writeUInt32LE(offset, entryOffset + 12);
    offset += image.data.length;
  }

  fs.writeFileSync(TARGET_ICO, Buffer.concat([header, ...images.map((image) => image.data)]));
};

const generateIcns = async () => {
  if (process.platform !== "darwin") {
    return;
  }

  const iconsetDir = fs.mkdtempSync(path.join(os.tmpdir(), "krabbyclaw-iconset-"));
  const iconsetPath = `${iconsetDir}.iconset`;
  fs.renameSync(iconsetDir, iconsetPath);

  try {
    for (const size of ICONSET_SIZES) {
      fs.writeFileSync(path.join(iconsetPath, `icon_${size}x${size}.png`), await renderSourcePng(size));
      fs.writeFileSync(path.join(iconsetPath, `icon_${size}x${size}@2x.png`), await renderSourcePng(size * 2));
    }

    try {
      runTool("iconutil", ["-c", "icns", iconsetPath, "-o", TARGET_ICNS]);
    } catch (error) {
      if (fs.existsSync(TARGET_ICNS)) {
        console.warn(`iconutil failed; keeping existing ${TARGET_ICNS}`);
      } else {
        throw error;
      }
    }
  } finally {
    fs.rmSync(iconsetPath, { recursive: true, force: true });
  }
};

const main = async () => {
  ensureSourceIcon(SOURCE_SVG);
  fs.mkdirSync(DESKTOP_ASSETS_DIR, { recursive: true });
  fs.writeFileSync(TARGET_PNG, await renderSourcePng(PNG_SIZE));
  await writeIco();
  await generateIcns();
};

if (require.main === module) {
  main().catch((error) => {
    console.error(error.message ?? error);
    process.exit(1);
  });
}

module.exports = {
  DESKTOP_ASSETS_DIR,
  ICO_SIZES,
  PNG_SIZE,
  SOURCE_SVG,
  TARGET_ICNS,
  TARGET_ICO,
  TARGET_PNG,
  generateIcns,
  main,
  renderSourcePng,
  writeIco,
};
