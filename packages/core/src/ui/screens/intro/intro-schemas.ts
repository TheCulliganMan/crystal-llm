// ASM reference: engine/movie/intro.asm

export type RGBColor = [number, number, number];
export type Palette = RGBColor[];

export class IntroSprite {
  constructor(
    public x: number,
    public y: number,
    public anim_id: number
  ) {}

  oam_attr = 0;
  gfx_name = "unowns";
  anim_frame = 0;
  jumptable_index = 0;
  frame_timer = 0;
  frameset_step = 0;
  start_delay = 0;
  x_offset = 0;
  y_offset = 0;
  var1 = 0;
  var2 = 0;
  var3 = 0;
  var4 = 0;
  frameset = 0;
  frame_counter = 0;
  palette_override: number | null = null;
  frameset_name: string | null = null;
  object_name: string | null = null;
  anim_function: string | null = null;
  current_oam_set: string | null = null;
  attr_flags = 0;
}

function generateBwFade(): RGBColor[] {
  const colors: RGBColor[] = [];
  for (let hue = 0; hue < 32; hue++) {
    colors.push([hue * 8, hue * 8, hue * 8]);
  }
  return colors;
}

function generateBlackLightBlue(): RGBColor[] {
  const palette: RGBColor[] = [];
  for (let hue = 0; hue < 32; hue++) {
    palette.push([0, Math.floor(hue / 2) * 8, hue * 8]);
  }
  return palette;
}

function generateBlackBlue(): RGBColor[] {
  const palette: RGBColor[] = [];
  for (let hue = 0; hue < 32; hue++) {
    palette.push([0, 0, hue * 8]);
  }
  return palette;
}

export const BW_FADE_TABLE = generateBwFade();
export const BLACK_LIGHT_BLUE_FADE = generateBlackLightBlue();
export const BLACK_BLUE_FADE = generateBlackBlue();

function generateFastFade(): RGBColor[] {
  const values: RGBColor[] = [];
  let hue = 31;
  for (let i = 0; i < 8; i++) {
    values.push([hue * 8, hue * 8, hue * 8]);
    hue -= 1;
    values.push([hue * 8, hue * 8, hue * 8]);
    hue -= 2;
  }
  return values;
}

function generateSlowFade(): RGBColor[] {
  const values: RGBColor[] = [];
  let hue = 31;
  for (let i = 0; i < 16; i++) {
    values.push([hue * 8, hue * 8, hue * 8]);
    hue -= 1;
  }
  return values;
}

export const FAST_FADE_PALETTES = generateFastFade();
export const SLOW_FADE_PALETTES = generateSlowFade();

// Legacy frameset mapping retained for compatibility with the current intro implementation.
export const UNOWN_FRAMESET: Record<number, Array<[number, number, number?]>> = {
  1: [
    [0, 3],
    [1, 3],
    [2, 7],
    [-1, 0],
  ],
  2: [
    [0, 3, 0x20],
    [1, 3, 0x20],
    [2, 7, 0x20],
    [-1, 0],
  ],
  3: [
    [0, 3, 0x40],
    [1, 3, 0x40],
    [2, 7, 0x40],
    [-1, 0],
  ],
  4: [
    [0, 3, 0x60],
    [1, 3, 0x60],
    [2, 7, 0x60],
    [-1, 0],
  ],
};
