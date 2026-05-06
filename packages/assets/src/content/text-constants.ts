// Shared constants for special font glyphs.

// The "Poké" ligature occupies a single tile in the Game Boy font.
export const POKE_GLYPH = '#';

// Convenience tokens for common words built from special glyphs.
export const POKEMON_WORD = `${POKE_GLYPH}MON`;

// Private-use placeholders for other single-tile ligatures and icons.
// These values mirror the Python implementation and disassembly control codes.
export const PKMN_GLYPH = '\uE100';
export const PC_GLYPH = '\uE101';
export const TM_GLYPH = '\uE102';
export const TRAINER_GLYPH = '\uE103';
export const ROCKET_GLYPH = '\uE104';
export const PK_GLYPH = '\uE105';
export const MN_GLYPH = '\uE106';
export const DOT_GLYPH = '\uE107';
export const PO_GLYPH = '\uE108';
export const KE_GLYPH = '\uE109';
export const LV_GLYPH = '\uE10A';
export const ID_GLYPH = '\uE10B';

// Control-code text snippets that expand to regular glyphs.
export const SIX_DOTS_TEXT = '……';
