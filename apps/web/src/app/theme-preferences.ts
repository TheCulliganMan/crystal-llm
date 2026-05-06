export const THEME_STORAGE_KEY = "pokecrystal.play.theme";
export const BRAND_THEME_STORAGE_KEY = "pokecrystal.play.brandTheme";

export const THEME_KEYS = ["morning", "day", "night"] as const;
export type ThemeKey = (typeof THEME_KEYS)[number];

export const BRAND_THEME_KEYS = [
  "krabby",
  "kingler",
  "heracross",
  "gligar",
  "scizor",
  "sneasel",
  "teddiursa",
  "ursaring",
  "totodile",
  "croconaw",
  "feraligatr",
  "pinsir",
] as const;
export type BrandThemeKey = (typeof BRAND_THEME_KEYS)[number];

export const isThemeKey = (value: unknown): value is ThemeKey =>
  typeof value === "string" && (THEME_KEYS as readonly string[]).includes(value);

export const isBrandThemeKey = (value: unknown): value is BrandThemeKey =>
  typeof value === "string" && (BRAND_THEME_KEYS as readonly string[]).includes(value);
