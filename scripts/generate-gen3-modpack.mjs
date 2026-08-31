#!/usr/bin/env node

import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import sharp from "sharp";

const EXPECTED_SOURCE_COMMIT = "c65e93f20a5275ab03b07d6f6411096a82a60ffd";
const REPOSITORY_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUTPUT_ROOT = path.join(REPOSITORY_ROOT, "modpacks", "gen3");
const DATA_PATH = path.join(OUTPUT_ROOT, "data.json");
const ASSET_ROOT = path.join(OUTPUT_ROOT, "assets");

const sourceArgumentIndex = process.argv.indexOf("--source");
if (sourceArgumentIndex < 0 || !process.argv[sourceArgumentIndex + 1]) {
  throw new Error("usage: node scripts/generate-gen3-modpack.mjs --source <pinned-pokeemerald-checkout>");
}
const sourceRoot = path.resolve(process.argv[sourceArgumentIndex + 1]);

const sourceCommit = execFileSync("git", ["-C", sourceRoot, "rev-parse", "HEAD"], {
  encoding: "utf8",
}).trim();
if (sourceCommit !== EXPECTED_SOURCE_COMMIT) {
  throw new Error(`pokeemerald source commit ${sourceCommit} does not match ${EXPECTED_SOURCE_COMMIT}`);
}

const readSource = (...segments) => fs.readFileSync(path.join(sourceRoot, ...segments), "utf8");
const readRepositoryJson = (...segments) =>
  JSON.parse(fs.readFileSync(path.join(REPOSITORY_ROOT, ...segments), "utf8"));

const pokedexConstants = readSource("include", "constants", "pokedex.h");
const nationalSpecies = [...pokedexConstants.matchAll(/^\s*NATIONAL_DEX_([A-Z0-9_]+),/gm)].map(
  (match) => match[1]
);
const firstGen3 = nationalSpecies.indexOf("TREECKO");
const lastGen3 = nationalSpecies.indexOf("DEOXYS");
if (firstGen3 < 0 || lastGen3 - firstGen3 + 1 !== 135) {
  throw new Error("pinned source does not expose the expected 135-species Gen 3 National Dex range");
}
const speciesIds = nationalSpecies.slice(firstGen3, lastGen3 + 1);
const nationalDexSpeciesIds = nationalSpecies.slice(1, lastGen3 + 1);
if (nationalDexSpeciesIds.length !== 386) {
  throw new Error("pinned source does not expose the expected 386-species National Dex");
}

const baseMoves = new Set(
  Object.keys(readRepositoryJson("packages", "assets", "src", "data", "moves-data.json"))
);
const baseMoveByCompactId = new Map([...baseMoves].map((moveId) => [moveId.replaceAll("_", ""), moveId]));
const canonicalMove = (sourceMoveId) =>
  ({ FEINT_ATTACK: "FAINT_ATTACK" })[sourceMoveId] ??
  baseMoveByCompactId.get(sourceMoveId.replaceAll("_", "")) ??
  null;
const baseItems = new Set(
  fs
    .readdirSync(path.join(REPOSITORY_ROOT, "apps", "web", "assets", "data", "content-packs", "core-modular", "items"))
    .filter((name) => name.endsWith(".json"))
    .map((name) => name.slice(0, -5))
);

const speciesInfoSource = readSource("src", "data", "pokemon", "species_info.h");
const tmhmSource = readSource("src", "data", "pokemon", "tmhm_learnsets.h");
const levelUpSource = readSource("src", "data", "pokemon", "level_up_learnsets.h");
const evolutionSource = readSource("src", "data", "pokemon", "evolution.h");
const pokedexEntrySource = readSource("src", "data", "pokemon", "pokedex_entries.h");
const pokedexTextSource = readSource("src", "data", "pokemon", "pokedex_text.h");

const requiredMatch = (source, expression, label) => {
  const match = source.match(expression);
  if (!match) throw new Error(`missing ${label} in pinned pokeemerald source`);
  return match;
};

const speciesBlock = (speciesId) =>
  requiredMatch(
    speciesInfoSource,
    new RegExp(`\\[SPECIES_${speciesId}\\]\\s*=\\s*\\{([\\s\\S]*?)\\n\\s*\\},?(?:\\n|$)`),
    `${speciesId} species info`
  )[1];

const numericField = (block, field, speciesId) =>
  Number(requiredMatch(block, new RegExp(`\\.${field}\\s*=\\s*(\\d+)`), `${speciesId}.${field}`)[1]);

const sourceTokenField = (block, field, speciesId) =>
  requiredMatch(block, new RegExp(`\\.${field}\\s*=\\s*([A-Z0-9_().]+)`), `${speciesId}.${field}`)[1];

const primaryAbility = (block, speciesId) =>
  requiredMatch(
    block,
    /\.abilities\s*=\s*\{\s*ABILITY_([A-Z0-9_]+)\s*,/,
    `${speciesId}.abilities`
  )[1];

const crystalSpeciesId = (speciesId) =>
  ({ FARFETCHD: "FARFETCH_D", MR_MIME: "MR__MIME" })[speciesId] ?? speciesId;

const mappedType = (sourceType) => (sourceType === "PSYCHIC" ? "PSYCHIC_TYPE" : sourceType);
const mappedEggGroup = (sourceGroup) =>
  ({ GRASS: "PLANT", HUMANOID: "HUMANSHAPE", AMORPHOUS: "INDETERMINATE", NO_EGGS_DISCOVERED: "NONE" })[
    sourceGroup
  ] ?? sourceGroup;

const genderRatio = (token) => {
  if (token === "MON_GENDERLESS") return 255;
  if (token === "MON_MALE") return 0;
  if (token === "MON_FEMALE") return 254;
  const percent = requiredMatch(token, /^PERCENT_FEMALE\(([\d.]+)\)$/, "gender ratio")[1];
  return Math.min(254, Math.floor((Number(percent) * 255) / 100));
};

const sourceItem = (block, field, speciesId) => {
  const token = sourceTokenField(block, field, speciesId).replace(/^ITEM_/, "");
  return token !== "NONE" && baseItems.has(token) ? token : null;
};

const pascalSpecies = (speciesId) =>
  speciesId
    .toLowerCase()
    .split("_")
    .map((part) => part[0].toUpperCase() + part.slice(1))
    .join("");

const levelUpMoves = (speciesId) => {
  const symbol = `s${pascalSpecies(speciesId)}LevelUpLearnset`;
  const block = requiredMatch(
    levelUpSource,
    new RegExp(`static const u16 ${symbol}\\[\\] = \\{([\\s\\S]*?)LEVEL_UP_END`),
    `${speciesId} level-up learnset`
  )[1];
  const entries = [...block.matchAll(/LEVEL_UP_MOVE\(\s*(\d+),\s*MOVE_([A-Z0-9_]+)\)/g)]
    .map((match) => [Number(match[1]), canonicalMove(match[2])])
    .filter(([, moveId]) => moveId !== null);
  if (entries.length === 0) {
    throw new Error(`${speciesId} has no Crystal-supported move in its source level-up learnset`);
  }
  return entries;
};

const tmhmMoves = (speciesId) => {
  const block = requiredMatch(
    tmhmSource,
    new RegExp(`\\[SPECIES_${speciesId}\\]\\s*=\\s*\\{\\s*\\.learnset\\s*=\\s*\\{([\\s\\S]*?)\\}\\s*\\}`),
    `${speciesId} TM/HM learnset`
  )[1];
  return [
    ...new Set(
      [...block.matchAll(/\.([A-Z0-9_]+)\s*=\s*TRUE/g)]
        .map((match) => canonicalMove(match[1]))
        .filter(Boolean)
    ),
  ];
};

const pokedexEntry = (speciesId) => {
  const block = requiredMatch(
    pokedexEntrySource,
    new RegExp(`\\[NATIONAL_DEX_${speciesId}\\]\\s*=\\s*\\{([\\s\\S]*?)\\n\\s*\\},`),
    `${speciesId} Pokedex entry`
  )[1];
  const classification = requiredMatch(block, /\.categoryName\s*=\s*_\("([^"]+)"\)/, `${speciesId} category`)[1];
  const heightDecimeters = numericField(block, "height", speciesId);
  const weightHectograms = numericField(block, "weight", speciesId);
  const textSymbol = requiredMatch(block, /\.description\s*=\s*([A-Za-z0-9_]+)/, `${speciesId} Pokedex text`)[1];
  const textBlock = requiredMatch(
    pokedexTextSource,
    new RegExp(`const u8 ${textSymbol}\\[\\] = _\\(([\\s\\S]*?)\\);`),
    `${speciesId} Pokedex text body`
  )[1];
  const description = [...textBlock.matchAll(/"((?:[^"\\]|\\.)*)"/g)]
    .map((match) => JSON.parse(`"${match[1]}"`))
    .join("")
    .replace(/\n/g, " ")
    .replace(/POKéMON/g, "Pokemon")
    .replace(/\s+/g, " ")
    .trim();
  const totalInches = Math.max(1, Math.round((heightDecimeters * 10) / 2.54));
  return {
    species: speciesId,
    classification,
    heightDigits: Math.floor(totalInches / 12) * 100 + (totalInches % 12),
    weightDigits: Math.max(1, Math.round(weightHectograms * 0.2204622622)),
    pages: [description],
  };
};

const supportedEvolutionEntry = (method, parameter, targetSpecies) => {
  const empty = { species: targetSpecies, level: null, item: null, held_item: null, happiness: null, stat_ratio: null };
  if (method === "EVO_LEVEL" || method === "EVO_LEVEL_NINJASK" || method === "EVO_LEVEL_SILCOON") {
    return { ...empty, method: "LEVEL", level: Number(parameter) };
  }
  if (method === "EVO_FRIENDSHIP") return { ...empty, method: "HAPPINESS", happiness: "TR_ANYTIME" };
  if (method === "EVO_ITEM") {
    const item = parameter.replace(/^ITEM_/, "");
    return baseItems.has(item) ? { ...empty, method: "ITEM", item } : null;
  }
  if (method === "EVO_TRADE") return { ...empty, method: "TRADE" };
  if (method === "EVO_TRADE_ITEM") {
    const heldItem = parameter.replace(/^ITEM_/, "");
    return baseItems.has(heldItem) ? { ...empty, method: "TRADE", held_item: heldItem } : null;
  }
  return null;
};

const evolutionsFor = (speciesId) => {
  const blockMatch = evolutionSource.match(
    new RegExp(
      `\\[SPECIES_${speciesId}\\]\\s*=\\s*([\\s\\S]*?)(?=\\n\\s*\\[SPECIES_|\\n\\};)`
    )
  );
  if (!blockMatch) return [];
  return [...blockMatch[1].matchAll(/\{(EVO_[A-Z0-9_]+),\s*([^,}]+),\s*SPECIES_([A-Z0-9_]+)\}/g)]
    .map((match) => supportedEvolutionEntry(match[1], match[2].trim(), match[3]))
    .filter(Boolean);
};

const menuIconFor = (type1, type2) => {
  const icons = {
    BUG: "ICON_BUG",
    DARK: "ICON_FOX",
    DRAGON: "ICON_SERPENT",
    ELECTRIC: "ICON_PIKACHU",
    FIGHTING: "ICON_FIGHTER",
    FIRE: "ICON_CHARMANDER",
    FLYING: "ICON_BIRD",
    GHOST: "ICON_GHOST",
    GRASS: "ICON_ODDISH",
    GROUND: "ICON_GEODUDE",
    ICE: "ICON_LAPRAS",
    NORMAL: "ICON_FOX",
    POISON: "ICON_BLOB",
    PSYCHIC_TYPE: "ICON_HUMANSHAPE",
    ROCK: "ICON_GEODUDE",
    STEEL: "ICON_MONSTER",
    WATER: "ICON_FISH",
  };
  return icons[type1] ?? icons[type2] ?? "ICON_MONSTER";
};

const pokemon = {};
const abilities = Object.fromEntries(
  nationalDexSpeciesIds.map((speciesId) => {
    const block = speciesBlock(speciesId);
    return [crystalSpeciesId(speciesId), primaryAbility(block, speciesId)];
  })
);
const learnsets = {};
const evolutions = {};
const pokedexEntries = {};
const frontpicAnimations = {};
const menuIcons = {};
const pokemonCries = {};

for (const [offset, speciesId] of speciesIds.entries()) {
  const block = speciesBlock(speciesId);
  const types = requiredMatch(block, /\.types\s*=\s*\{\s*TYPE_([A-Z0-9_]+),\s*TYPE_([A-Z0-9_]+)\s*\}/, `${speciesId}.types`);
  const eggGroups = requiredMatch(
    block,
    /\.eggGroups\s*=\s*\{\s*EGG_GROUP_([A-Z0-9_]+),\s*EGG_GROUP_([A-Z0-9_]+),?\s*\}/,
    `${speciesId}.eggGroups`
  );
  const type1 = mappedType(types[1]);
  const type2 = mappedType(types[2]);
  const speciesLearnset = levelUpMoves(speciesId);
  pokemon[speciesId] = {
    id: speciesId,
    int_id: 252 + offset,
    base_stats: {
      hp: numericField(block, "baseHP", speciesId),
      attack: numericField(block, "baseAttack", speciesId),
      defense: numericField(block, "baseDefense", speciesId),
      speed: numericField(block, "baseSpeed", speciesId),
      special_attack: numericField(block, "baseSpAttack", speciesId),
      special_defense: numericField(block, "baseSpDefense", speciesId),
    },
    type1,
    type2,
    catch_rate: numericField(block, "catchRate", speciesId),
    base_exp: numericField(block, "expYield", speciesId),
    item1: sourceItem(block, "itemCommon", speciesId),
    item2: sourceItem(block, "itemRare", speciesId),
    gender_ratio: genderRatio(sourceTokenField(block, "genderRatio", speciesId)),
    unknown1: 0,
    step_cycles_to_hatch: numericField(block, "eggCycles", speciesId),
    unknown2: 0,
    growth_rate: sourceTokenField(block, "growthRate", speciesId),
    egg_group1: `EGG_${mappedEggGroup(eggGroups[1])}`,
    egg_group2: `EGG_${mappedEggGroup(eggGroups[2])}`,
    tmhm_learnset: tmhmMoves(speciesId),
    ability: primaryAbility(block, speciesId),
    pic_size: 0x77,
    front_pic: 0,
    back_pic: 0,
    weight: pokedexEntry(speciesId).weightDigits,
  };
  learnsets[speciesId] = speciesLearnset;
  evolutions[speciesId] = evolutionsFor(speciesId);
  pokedexEntries[speciesId] = pokedexEntry(speciesId);
  frontpicAnimations[speciesId] = { commands: [{ kind: "endanim" }] };
  menuIcons[speciesId] = menuIconFor(type1, type2);
  pokemonCries[speciesId] = { cry: `CRY_MON_${speciesId}`, pitch: 0, length: 0 };
}

const readJascPalette = (palettePath) => {
  const lines = fs.readFileSync(palettePath, "utf8").trim().split(/\r?\n/);
  if (lines[0] !== "JASC-PAL" || Number(lines[2]) !== 16) throw new Error(`invalid JASC palette ${palettePath}`);
  return lines.slice(3).map((line) => line.split(/\s+/).map(Number));
};

const rgbDistance = (left, right) =>
  (left[0] - right[0]) ** 2 + (left[1] - right[1]) ** 2 + (left[2] - right[2]) ** 2;

const gbc15 = ([red, green, blue]) =>
  Math.round(red / 8) | (Math.round(green / 8) << 5) | (Math.round(blue / 8) << 10);

const writeGbcpal = (filePath, colors) => {
  const bytes = Buffer.alloc(8);
  colors.forEach((color, index) => bytes.writeUInt16LE(gbc15(color) & 0x7fff, index * 2));
  fs.writeFileSync(filePath, bytes);
};

const writeAsmPalette = (filePath, colors) => {
  const components = colors.map((color) => color.map((component) => Math.round(component / 8)).join(", "));
  fs.writeFileSync(filePath, `${components.map((value) => `\tRGB ${value}`).join("\n")}\n`);
};

const write2bpp = (filePath, raw, colors) => {
  const bytes = [];
  for (let tileY = 0; tileY < 7; tileY += 1) {
    for (let tileX = 0; tileX < 7; tileX += 1) {
      for (let row = 0; row < 8; row += 1) {
        let low = 0;
        let high = 0;
        for (let column = 0; column < 8; column += 1) {
          const pixel = ((tileY * 8 + row) * 56 + tileX * 8 + column) * 4;
          const color = [raw[pixel], raw[pixel + 1], raw[pixel + 2]];
          const paletteIndex = raw[pixel + 3] === 0
            ? 0
            : colors.reduce(
                (best, candidate, index) =>
                  rgbDistance(color, candidate) < best.distance
                    ? { index, distance: rgbDistance(color, candidate) }
                    : best,
                { index: 0, distance: Number.POSITIVE_INFINITY }
              ).index;
          low |= (paletteIndex & 1) << (7 - column);
          high |= ((paletteIndex >> 1) & 1) << (7 - column);
        }
        bytes.push(low, high);
      }
    }
  }
  fs.writeFileSync(filePath, Buffer.from(bytes));
};

const convertSprite = async (speciesId) => {
  const stem = speciesId.toLowerCase();
  const sourceDir = path.join(
    sourceRoot,
    "graphics",
    "pokemon",
    stem,
    ...(speciesId === "CASTFORM" ? ["normal"] : [])
  );
  const outputDir = path.join(ASSET_ROOT, "gfx", "pokemon", stem);
  fs.mkdirSync(outputDir, { recursive: true });
  const normalPalette = readJascPalette(path.join(sourceDir, "normal.pal"));
  const shinyPalette = readJascPalette(path.join(sourceDir, "shiny.pal"));
  const frontImage = sharp(path.join(sourceDir, "front.png")).resize(56, 56, { kernel: "nearest" }).ensureAlpha();
  const { data: frontRaw } = await frontImage.clone().raw().toBuffer({ resolveWithObject: true });
  const frequencies = new Map();
  for (let offset = 0; offset < frontRaw.length; offset += 4) {
    if (frontRaw[offset + 3] === 0) continue;
    const pixel = [frontRaw[offset], frontRaw[offset + 1], frontRaw[offset + 2]];
    const paletteIndex = normalPalette.reduce(
      (best, color, index) =>
        rgbDistance(pixel, color) < best.distance ? { index, distance: rgbDistance(pixel, color) } : best,
      { index: 0, distance: Number.POSITIVE_INFINITY }
    ).index;
    if (paletteIndex !== 0) frequencies.set(paletteIndex, (frequencies.get(paletteIndex) ?? 0) + 1);
  }
  const selectedIndices = [0, ...[...frequencies.entries()].sort((a, b) => b[1] - a[1]).slice(0, 3).map(([index]) => index)];
  while (selectedIndices.length < 4) selectedIndices.push(selectedIndices[selectedIndices.length - 1] ?? 0);
  const selectedNormal = selectedIndices.map((index) => normalPalette[index]);
  const selectedShiny = selectedIndices.map((index) => shinyPalette[index]);
  await frontImage.clone().png().toFile(path.join(outputDir, "front.png"));
  await sharp(path.join(sourceDir, "back.png"))
    .resize(56, 56, { kernel: "nearest" })
    .ensureAlpha()
    .png()
    .toFile(path.join(outputDir, "back.png"));
  fs.writeFileSync(path.join(outputDir, "front.dimensions"), Buffer.from([0x77]));
  write2bpp(path.join(outputDir, "front.2bpp"), frontRaw, selectedNormal);
  writeGbcpal(path.join(outputDir, "normal.gbcpal"), selectedNormal);
  writeAsmPalette(path.join(outputDir, "shiny.pal"), selectedShiny);
};

const wavToCanonicalPcm = (wavBytes, label) => {
  if (wavBytes.toString("ascii", 0, 4) !== "RIFF" || wavBytes.toString("ascii", 8, 12) !== "WAVE") {
    throw new Error(`${label} is not a RIFF WAVE file`);
  }
  let offset = 12;
  let format = null;
  let samples = null;
  while (offset + 8 <= wavBytes.length) {
    const chunkId = wavBytes.toString("ascii", offset, offset + 4);
    const chunkLength = wavBytes.readUInt32LE(offset + 4);
    const chunk = wavBytes.subarray(offset + 8, offset + 8 + chunkLength);
    if (chunkId === "fmt ") {
      format = {
        encoding: chunk.readUInt16LE(0),
        channels: chunk.readUInt16LE(2),
        sampleRate: chunk.readUInt32LE(4),
        bits: chunk.readUInt16LE(14),
      };
    } else if (chunkId === "data") samples = chunk;
    offset += 8 + chunkLength + (chunkLength & 1);
  }
  if (!format || !samples || format.encoding !== 1 || format.channels !== 1 || ![8, 16].includes(format.bits)) {
    throw new Error(`${label} must be mono 8-bit or 16-bit PCM`);
  }
  const sourceLength = samples.length / (format.bits / 8);
  const sampleAt = (index) =>
    format.bits === 8 ? (samples[index] - 128) * 256 : samples.readInt16LE(index * 2);
  const outputFrames = Math.max(1, Math.round((sourceLength * 22050) / format.sampleRate));
  const output = Buffer.alloc(outputFrames * 4);
  for (let frame = 0; frame < outputFrames; frame += 1) {
    const position = (frame * format.sampleRate) / 22050;
    const left = Math.min(sourceLength - 1, Math.floor(position));
    const right = Math.min(sourceLength - 1, left + 1);
    const fraction = position - left;
    const sample = Math.round(sampleAt(left) * (1 - fraction) + sampleAt(right) * fraction);
    output.writeInt16LE(sample, frame * 4);
    output.writeInt16LE(sample, frame * 4 + 2);
  }
  return output;
};

fs.rmSync(ASSET_ROOT, { recursive: true, force: true });
fs.mkdirSync(path.join(ASSET_ROOT, "audio"), { recursive: true });
for (const speciesId of speciesIds) {
  await convertSprite(speciesId);
  const stem = speciesId.toLowerCase();
  const wav = fs.readFileSync(path.join(sourceRoot, "sound", "direct_sound_samples", "cries", `${stem}.wav`));
  fs.writeFileSync(
    path.join(ASSET_ROOT, "audio", `CRY_MON_${speciesId}.pcm`),
    wavToCanonicalPcm(wav, speciesId)
  );
}

const output = {
  schema_version: 1,
  metadata: {
    id: "gen3",
    name: "Generation 3 Pokemon",
    version: "1.0.0",
    author: null,
    description: "Adds National Dex species 252 through 386 with Emerald species data and Crystal-compatible learnsets.",
  },
  source: { repository: "https://github.com/pret/pokeemerald", commit: EXPECTED_SOURCE_COMMIT },
  abilities,
  pokemon,
  learnsets,
  evolutions,
  menu_icons: menuIcons,
  pokedex_entries: pokedexEntries,
  pokemon_frontpic_anim: frontpicAnimations,
  pokemon_cries: pokemonCries,
};
fs.mkdirSync(OUTPUT_ROOT, { recursive: true });
fs.writeFileSync(DATA_PATH, `${JSON.stringify(output, null, 2)}\n`);
console.log(`generated ${speciesIds.length} Gen 3 species in ${path.relative(REPOSITORY_ROOT, OUTPUT_ROOT)}`);
