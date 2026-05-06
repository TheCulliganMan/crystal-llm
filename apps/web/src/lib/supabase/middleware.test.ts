export {};

const originalEnv = process.env;

type MockRequest = {
  cookies: {
    getAll: jest.Mock;
    set: jest.Mock;
  };
};

const buildRequest = (): MockRequest => ({
  cookies: {
    getAll: jest.fn(() => []),
    set: jest.fn(),
  },
});

describe("supabase middleware", () => {
  beforeEach(() => {
    jest.resetModules();
    process.env = { ...originalEnv };
    process.env.NEXT_PUBLIC_SUPABASE_URL = "https://example.supabase.co";
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY = "anon-key";
  });

  afterEach(() => {
    process.env = originalEnv;
    jest.restoreAllMocks();
  });

  it("falls back to pass-through when supabase client creation throws", async () => {
    const nextMock = jest.fn(({ request }) => ({ request, cookies: { set: jest.fn() } }));
    jest.doMock("next/server", () => ({
      NextResponse: { next: nextMock },
    }));
    jest.doMock("@supabase/ssr", () => ({
      createServerClient: jest.fn(() => {
        throw new Error("invalid supabase url");
      }),
    }));
    const warnSpy = jest.spyOn(console, "warn").mockImplementation(() => undefined);
    const { updateSupabaseSession } = require("./middleware");

    const request = buildRequest();
    const response = await updateSupabaseSession(request);

    expect(response.request).toBe(request);
    expect(nextMock).toHaveBeenCalledTimes(2);
    expect(warnSpy).toHaveBeenCalledWith(
      "[middleware] failed to refresh supabase session; continuing request",
      expect.any(Error)
    );
  });

  it("falls back to pass-through when auth refresh throws", async () => {
    const nextMock = jest.fn(({ request }) => ({ request, cookies: { set: jest.fn() } }));
    jest.doMock("next/server", () => ({
      NextResponse: { next: nextMock },
    }));
    jest.doMock("@supabase/ssr", () => ({
      createServerClient: jest.fn(() => ({
        auth: {
          getUser: jest.fn(async () => {
            throw new Error("network down");
          }),
        },
      })),
    }));
    const warnSpy = jest.spyOn(console, "warn").mockImplementation(() => undefined);
    const { updateSupabaseSession } = require("./middleware");

    const request = buildRequest();
    const response = await updateSupabaseSession(request);

    expect(response.request).toBe(request);
    expect(nextMock).toHaveBeenCalledTimes(2);
    expect(warnSpy).toHaveBeenCalledWith(
      "[middleware] failed to refresh supabase session; continuing request",
      expect.any(Error)
    );
  });
});
