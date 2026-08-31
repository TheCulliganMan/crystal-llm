import { POST } from "./route";
import { createSupabaseServerClient, createSupabaseServiceRoleClient } from "@/lib/supabase/server";
import { isSupabaseConfigured, isSupabaseServiceRoleConfigured } from "@/lib/supabase/env";

jest.mock("@/lib/supabase/server", () => ({
  createSupabaseServerClient: jest.fn(),
  createSupabaseServiceRoleClient: jest.fn(),
}));
jest.mock("@/lib/supabase/env", () => ({
  isSupabaseConfigured: jest.fn(() => true),
  isSupabaseServiceRoleConfigured: jest.fn(() => true),
}));

describe("chat reports route", () => {
  beforeEach(() => {
    jest.clearAllMocks();
    jest.mocked(isSupabaseConfigured).mockReturnValue(true);
    jest.mocked(isSupabaseServiceRoleConfigured).mockReturnValue(true);
  });

  it("stores an authenticated moderation report", async () => {
    const insert = jest.fn(async () => ({ error: null }));
    jest.mocked(createSupabaseServerClient).mockReturnValue({
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: "reporter" } } })) },
    } as any);
    jest.mocked(createSupabaseServiceRoleClient).mockReturnValue({
      from: jest.fn(() => ({ insert })),
    } as any);

    const response = await POST(new Request("http://localhost/api/multiplayer/chat/reports", {
      method: "POST",
      body: JSON.stringify({
        messageId: "m1", reportedUserId: "peer-1", playerName: "Leaf",
        channel: "trade", text: "spam",
      }),
    }));
    expect(response.status).toBe(200);
    expect(insert).toHaveBeenCalledWith(expect.objectContaining({
      reporter_user_id: "reporter", reported_user_id: "peer-1", message_id: "m1",
    }));
  });

  it("rejects self-reports and malformed channels", async () => {
    jest.mocked(createSupabaseServerClient).mockReturnValue({
      auth: { getUser: jest.fn(async () => ({ data: { user: { id: "reporter" } } })) },
    } as any);
    jest.mocked(createSupabaseServiceRoleClient).mockReturnValue({ from: jest.fn() } as any);
    const response = await POST(new Request("http://localhost/api/multiplayer/chat/reports", {
      method: "POST",
      body: JSON.stringify({ messageId: "m1", reportedUserId: "reporter", playerName: "Me", channel: "guild", text: "x" }),
    }));
    expect(response.status).toBe(400);
  });
});
