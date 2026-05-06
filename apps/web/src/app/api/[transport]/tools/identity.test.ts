import { registerIdentityHandler, whoAmIHandler } from "./identity";

const mockCreateSupabaseServiceRoleClient = jest.fn();

jest.mock("@/lib/supabase/server", () => ({
  createSupabaseServiceRoleClient: () => mockCreateSupabaseServiceRoleClient(),
}));

const makeSupabaseListResponse = (error: { message?: string; code?: string } | null) => {
  const query = {
    select: jest.fn().mockReturnThis(),
    eq: jest.fn().mockReturnThis(),
    order: jest.fn().mockResolvedValue({
      data: error ? null : [],
      error,
    }),
  };
  return {
    from: jest.fn().mockReturnValue(query),
  };
};

describe("whoAmIHandler", () => {
  beforeEach(() => {
    mockCreateSupabaseServiceRoleClient.mockReset();
  });

  it("returns an empty save list when game_saves table is missing", async () => {
    mockCreateSupabaseServiceRoleClient.mockReturnValue(
      makeSupabaseListResponse({
        code: "PGRST205",
        message: "Could not find the table 'public.game_saves' in the schema cache",
      })
    );

    const registerResponse = await registerIdentityHandler({});
    const registerPayload = JSON.parse(String(registerResponse.content?.[0]?.text ?? "{}")) as {
      token: string;
      playerId: string;
    };

    const response = await whoAmIHandler(
      {},
      {
        requestInfo: {
          headers: new Headers({
            authorization: `Bearer ${registerPayload.token}`,
          }),
        },
      }
    );

    const payload = JSON.parse(String(response.content?.[0]?.text ?? "{}")) as {
      playerId: string;
      saveSlots: { count: number; slots: unknown[] };
    };
    expect(payload.playerId).toBe(registerPayload.playerId);
    expect(payload.saveSlots.count).toBe(0);
    expect(payload.saveSlots.slots).toEqual([]);
  });

  it("throws when Supabase returns a different error", async () => {
    mockCreateSupabaseServiceRoleClient.mockReturnValue(
      makeSupabaseListResponse({
        code: "42501",
        message: "permission denied for table game_saves",
      })
    );

    const registerResponse = await registerIdentityHandler({});
    const registerPayload = JSON.parse(String(registerResponse.content?.[0]?.text ?? "{}")) as {
      token: string;
    };

    await expect(
      whoAmIHandler(
        {},
        {
          requestInfo: {
            headers: new Headers({
              authorization: `Bearer ${registerPayload.token}`,
            }),
          },
        }
      )
    ).rejects.toThrow("permission denied");
  });
});
