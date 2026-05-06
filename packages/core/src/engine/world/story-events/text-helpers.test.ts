describe("resolveText", () => {
  afterEach(() => {
    jest.resetModules();
    jest.clearAllMocks();
    jest.restoreAllMocks();
  });

  it("falls back to underscore-prefixed ASM labels", async () => {
    const asmGet = jest.fn((label: string) => {
      if (label === "_CaughtAskNicknameText") {
        return "Give a nickname to the newly caught POKEMON?";
      }
      return "";
    });

    jest.doMock("@pokecrystal/core/core/asm-text-loader", () => ({
      asmTextLoader: { get: asmGet },
    }));

    const { resolveText } = await import("./text-helpers");

    expect(resolveText(null, null, "CaughtAskNicknameText")).toBe(
      "Give a nickname to the newly caught POKEMON?"
    );
    expect(asmGet).toHaveBeenCalledWith("_CaughtAskNicknameText");
  });

  it("falls back to standard text when canonical ASM text is unavailable", async () => {
    const asmGet = jest.fn(() => "");

    jest.doMock("@pokecrystal/core/core/asm-text-loader", () => ({
      asmTextLoader: { get: asmGet },
    }));

    const { resolveText } = await import("./text-helpers");

    expect(resolveText(null, null, "DifficultBookshelfText")).toBe("It's full of\ndifficult books.");
  });

  it("throws instead of prettifying unresolved labels into synthetic text", async () => {
    const asmGet = jest.fn(() => "");

    jest.doMock("@pokecrystal/core/core/asm-text-loader", () => ({
      asmTextLoader: { get: asmGet },
    }));

    const { resolveText } = await import("./text-helpers");

    expect(() => resolveText(null, null, "TotallyMissingStoryText")).toThrow(
      "Missing ASM text for label 'TotallyMissingStoryText'."
    );
  });
});
