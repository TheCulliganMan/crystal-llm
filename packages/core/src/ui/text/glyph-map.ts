import {
  DOT_GLYPH,
  ID_GLYPH,
  KE_GLYPH,
  LV_GLYPH,
  MN_GLYPH,
  PC_GLYPH,
  PK_GLYPH,
  PKMN_GLYPH,
  PO_GLYPH,
  POKE_GLYPH,
  ROCKET_GLYPH,
  TM_GLYPH,
  TRAINER_GLYPH,
} from "@pokecrystal/assets/content/text-constants";

export type GlyphMap = Record<string, number>;

export function buildDefaultCharMap(): GlyphMap {
  const charMap: GlyphMap = {};

  // Space (explicitly loaded from space tile)
  charMap[" "] = 0x7f;

  // Uppercase letters (0x80-0x99)
  "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").forEach((char, i) => {
    charMap[char] = 0x80 + i;
  });

  // Lowercase letters (0xA0-0xB9)
  "abcdefghijklmnopqrstuvwxyz".split("").forEach((char, i) => {
    charMap[char] = 0xa0 + i;
  });

  // German umlauts present in the original font.
  charMap["\u00c4"] = 0xc0;
  charMap["\u00d6"] = 0xc1;
  charMap["\u00dc"] = 0xc2;
  charMap["\u00e4"] = 0xc3;
  charMap["\u00f6"] = 0xc4;
  charMap["\u00fc"] = 0xc5;

  // Numbers (0xF6-0xFF)
  for (let i = 0; i < 10; i += 1) {
    charMap[String(i)] = 0xf6 + i;
  }

  // Symbols and punctuation.
  charMap["("] = 0x9a;
  charMap[")"] = 0x9b;
  charMap[":"] = 0x9c;
  charMap[";"] = 0x9d;
  charMap["["] = 0x9e;
  charMap["]"] = 0x9f;
  charMap["\u250c"] = 0x79;
  charMap["\u2500"] = 0x7a;
  charMap["\u2510"] = 0x7b;
  charMap["\u2502"] = 0x7c;
  charMap["\u2514"] = 0x7d;
  charMap["\u2518"] = 0x7e;
  charMap["\u260e"] = 0x62;
  charMap["\u2014"] = 0x7a;
  charMap["\u2013"] = 0x7a;
  charMap["'"] = 0xe0;
  charMap["-"] = 0xe3;
  charMap["?"] = 0xe6;
  charMap["!"] = 0xe7;
  charMap["."] = 0xe8;
  charMap["\u00d7"] = 0xf1;
  charMap["&"] = 0xe9;
  charMap["/"] = 0xf3;
  charMap["_"] = 0x5f;
  charMap[","] = 0xf4;
  charMap["="] = 0x3d;
  charMap["+"] = 0x2b;
  charMap["\u00e9"] = 0xea;
  charMap["\u00c9"] = 0xea;
  charMap["\u00a5"] = 0xf0;
  charMap["\u2642"] = 0xef;
  charMap["\u2640"] = 0xf5;

  // Arrow glyphs used for menu cursors.
  charMap["\u25b6"] = 0xed;
  charMap["\u25b7"] = 0xec;
  charMap["\u25c0"] = 0x71;
  charMap["\u25bc"] = 0xee;
  charMap["\u25b2"] = 0x61;
  charMap["<"] = 0x71;
  charMap[">"] = 0xed;

  // Quotes (height strings use the closing quote tile).
  charMap['"'] = 0x73;

  // Single-tile ligatures and icon glyphs.
  charMap[PKMN_GLYPH] = 0x4a;
  charMap[PC_GLYPH] = 0x5b;
  charMap[TM_GLYPH] = 0x5c;
  charMap[TRAINER_GLYPH] = 0x5d;
  charMap[ROCKET_GLYPH] = 0x5e;
  charMap[POKE_GLYPH] = 0x54;
  charMap[PO_GLYPH] = 0x70;
  charMap[KE_GLYPH] = 0x71;
  charMap[LV_GLYPH] = 0x6e;
  charMap[ID_GLYPH] = 0x73;
  charMap[PK_GLYPH] = 0xe1;
  charMap[MN_GLYPH] = 0xe2;
  charMap[DOT_GLYPH] = 0xf2;
  charMap["\u2026"] = 0x75;

  return charMap;
}
