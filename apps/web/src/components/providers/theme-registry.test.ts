import { __test_only__baseThemeOptions } from "./theme-registry";

describe("theme-registry typography colors", () => {
  it("keeps typography tied to css theme tokens", () => {
    const typography = __test_only__baseThemeOptions.typography ?? {};
    expect(typography).toMatchObject({
      fontFamily: "var(--font-space-grotesk), sans-serif",
    });
    expect((typography as { button?: Record<string, unknown> }).button).toMatchObject({
      textTransform: "none",
      fontWeight: 700,
    });
  });

  it("keeps paper corner radius settings consistent", () => {
    expect(__test_only__baseThemeOptions.shape).toMatchObject({
      borderRadius: 14,
    });
  });
});
