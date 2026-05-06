/** @jest-environment jsdom */

const mockDynamic = jest.fn(() => () => null);

jest.mock("next/dynamic", () => ({
  __esModule: true,
  default: (...args: unknown[]) => mockDynamic(...args),
}));

describe("play-panel lazy loader", () => {
  it("disables SSR for hydration stability", async () => {
    await import("./play-panel.lazy");
    expect(mockDynamic).toHaveBeenCalledTimes(1);
    const options = mockDynamic.mock.calls[0]?.[1] as { ssr?: boolean };
    expect(options?.ssr).toBe(false);
  });
});
