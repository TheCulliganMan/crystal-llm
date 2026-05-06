export {};

const originalEnv = process.env;

describe("supabase server client", () => {
  beforeEach(() => {
    jest.resetModules();
    process.env = { ...originalEnv };
    delete process.env.NEXT_PUBLIC_SUPABASE_URL;
    delete process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY;
    delete process.env.SUPABASE_SERVICE_ROLE_KEY;
  });

  afterEach(() => {
    process.env = originalEnv;
  });

  it("returns null when public config is missing", () => {
    jest.doMock("@supabase/ssr", () => ({
      createServerClient: jest.fn(() => ({ tag: "client" })),
    }));
    const { createSupabaseServerClient } = require("./server");
    const { createServerClient } = require("@supabase/ssr");
    expect(createSupabaseServerClient()).toBeNull();
    expect(createServerClient).not.toHaveBeenCalled();
  });

  it("creates a server client when public config is present", () => {
    process.env.NEXT_PUBLIC_SUPABASE_URL = "https://example.supabase.co";
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY = "anon-key";
    jest.doMock("@supabase/ssr", () => ({
      createServerClient: jest.fn(() => ({ tag: "client" })),
    }));
    const { createSupabaseServerClient } = require("./server");
    const { createServerClient } = require("@supabase/ssr");
    expect(createSupabaseServerClient()).toEqual({ tag: "client" });
    expect(createServerClient).toHaveBeenCalledWith(
      "https://example.supabase.co",
      "anon-key",
      expect.objectContaining({ cookies: expect.any(Object) })
    );
  });

  it("returns null when service role config is missing", () => {
    process.env.NEXT_PUBLIC_SUPABASE_URL = "https://example.supabase.co";
    jest.doMock("@supabase/ssr", () => ({
      createServerClient: jest.fn(() => ({ tag: "client" })),
    }));
    const { createSupabaseServiceRoleClient } = require("./server");
    const { createServerClient } = require("@supabase/ssr");
    expect(createSupabaseServiceRoleClient()).toBeNull();
    expect(createServerClient).not.toHaveBeenCalled();
  });
});
