import { BRAND_THEME_KEYS, isBrandThemeKey } from "./theme-preferences";

describe("theme-preferences brand themes", () => {
  it("accepts every configured brand theme key", () => {
    for (const key of BRAND_THEME_KEYS) {
      expect(isBrandThemeKey(key)).toBe(true);
    }
  });

  it("rejects unknown brand theme keys", () => {
    expect(isBrandThemeKey("gliscor")).toBe(false);
    expect(isBrandThemeKey("")).toBe(false);
    expect(isBrandThemeKey(null)).toBe(false);
  });
});
