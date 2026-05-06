/** @jest-environment jsdom */

const mockCreateSupabaseBrowserClient = jest.fn(() => ({ auth: {} }));
const mockIsSupabaseConfigured = jest.fn(() => true);

jest.mock("@/lib/supabase/browser", () => ({
  createSupabaseBrowserClient: () => mockCreateSupabaseBrowserClient(),
}));

jest.mock("@/lib/supabase/env", () => ({
  isSupabaseConfigured: () => mockIsSupabaseConfigured(),
}));

describe("register-browser-adapters", () => {
  beforeEach(() => {
    jest.resetModules();
    mockCreateSupabaseBrowserClient.mockClear();
    mockIsSupabaseConfigured.mockReset();
    mockIsSupabaseConfigured.mockReturnValue(true);
    window.history.replaceState({}, "", "/");
  });

  it("disables cloud save on the desktop route even when Supabase is configured", async () => {
    window.history.replaceState({}, "", "/desktop");

    const adapters = await import("@pokecrystal/core/adapters");
    adapters.resetCloudSaveAdapters();
    await import("./register-browser-adapters");

    expect(adapters.isBrowserCloudSaveConfigured()).toBe(false);
  });

  it("keeps cloud save available on non-desktop routes when Supabase is configured", async () => {
    const adapters = await import("@pokecrystal/core/adapters");
    adapters.resetCloudSaveAdapters();
    await import("./register-browser-adapters");

    expect(adapters.isBrowserCloudSaveConfigured()).toBe(true);
  });
});
