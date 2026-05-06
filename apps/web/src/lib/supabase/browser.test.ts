jest.mock("@supabase/supabase-js", () => ({
  createClient: jest.fn(() => ({ auth: { getUser: jest.fn() } })),
}));

describe("createSupabaseBrowserClient", () => {
  const originalEnv = { ...process.env };

  beforeEach(() => {
    jest.resetModules();
    process.env = {
      ...originalEnv,
      NEXT_PUBLIC_SUPABASE_URL: "https://example.supabase.co",
      NEXT_PUBLIC_SUPABASE_ANON_KEY: "anon-key",
    };
  });

  afterAll(() => {
    process.env = originalEnv;
  });

  it("reuses one browser client instance for repeated calls", async () => {
    const supabase = await import("@supabase/supabase-js");
    const mod = await import("./browser");

    const first = mod.createSupabaseBrowserClient();
    const second = mod.createSupabaseBrowserClient();

    expect(first).toBe(second);
    expect(supabase.createClient).toHaveBeenCalledTimes(1);
  });
});
