const mockIsSupabaseConfigured = jest.fn();
const mockGetUser = jest.fn();
const mockMaybeSingle = jest.fn();

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/lib/supabase/env", () => ({
    isSupabaseConfigured: mockIsSupabaseConfigured,
  }));
  jest.doMock("@/lib/supabase/server", () => ({
    createSupabaseServerClient: () => ({
      auth: {
        getUser: mockGetUser,
      },
      from: () => ({
        select: () => ({
          eq: () => ({
            eq: () => ({
              maybeSingle: mockMaybeSingle,
            }),
          }),
        }),
      }),
    }),
  }));
  return await import("./route");
};

describe("savegame API", () => {
  beforeEach(() => {
    mockIsSupabaseConfigured.mockReset();
    mockGetUser.mockReset();
    mockMaybeSingle.mockReset();
    mockIsSupabaseConfigured.mockReturnValue(true);
    mockGetUser.mockResolvedValue({ data: { user: { id: "user-1" } } });
  });

  it("returns 204 when no save exists for the requested slot", async () => {
    mockMaybeSingle.mockResolvedValue({ data: null, error: null });
    const { GET } = await loadRoute();

    const response = await GET(new Request("http://localhost/api/savegame?slot=savegame"));

    expect(response.status).toBe(204);
    expect(await response.text()).toBe("");
  });
});
