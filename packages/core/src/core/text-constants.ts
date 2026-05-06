/** ASM-derived constants for UI text and textbox layout. */

export const SCREEN_TILE_WIDTH = 20;
export const SCREEN_TILE_HEIGHT = 18;
export const TEXTBOX_BORDER_WIDTH = 2;
export const TEXTBOX_HEIGHT_TILES = 6;
export const TEXTBOX_INNER_HEIGHT_TILES =
  TEXTBOX_HEIGHT_TILES - TEXTBOX_BORDER_WIDTH;
export const TEXTBOX_Y_TILES = SCREEN_TILE_HEIGHT - TEXTBOX_HEIGHT_TILES;
export const TEXTBOX_INNER_Y_TILES = TEXTBOX_Y_TILES + TEXTBOX_BORDER_WIDTH;

export const TEXTBOX_DELAY_FAST_FRAMES = 1;
export const TEXTBOX_FAST_DELAY_FLAG = 1 << 0;
export const TEXTBOX_DELAY_FLAG = 1 << 1;

export {
  POKE_GLYPH,
  POKEMON_WORD,
  PKMN_GLYPH,
  PC_GLYPH,
  TM_GLYPH,
  TRAINER_GLYPH,
  ROCKET_GLYPH,
  PK_GLYPH,
  MN_GLYPH,
  DOT_GLYPH,
  PO_GLYPH,
  KE_GLYPH,
  LV_GLYPH,
  ID_GLYPH,
  SIX_DOTS_TEXT,
} from "@pokecrystal/assets/content/text-constants";
