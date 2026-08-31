const mockIsSupabaseConfigured = jest.fn();
const mockIsSupabaseServiceRoleConfigured = jest.fn();
const mockGetUser = jest.fn();

type ProfileRow = {
  id: string;
  handle: string;
  display_name: string | null;
  link_battle_wins: number;
  link_battle_losses: number;
  link_battle_rating: number;
  total_trades: number;
};

type MatchRow = {
  id: string;
  player1_id: string;
  player2_id: string;
  mode: "battle" | "trade" | "time_capsule";
  modpack_id: string;
  ranked: boolean;
  status: "waiting" | "active" | "completed" | "cancelled";
  channel_name: string;
  result: Record<string, unknown> | null;
  created_at: string;
  started_at: string | null;
  completed_at: string | null;
};

const buildServiceClient = () => {
  const state = {
    profiles: new Map<string, ProfileRow>(),
    matches: [] as MatchRow[],
    leaderboard: [] as Array<Record<string, unknown>>,
  };

  const selectProfile = (id: string) => ({
    data: state.profiles.get(id) ?? null,
    error: null,
  });

  const from = jest.fn((table: string) => {
    if (table === "arena_profiles") {
      return {
        select: () => ({
          eq: (_column: string, id: string) => ({
            maybeSingle: async () => selectProfile(id),
          }),
        }),
        insert: (payload: Partial<ProfileRow> & { id: string; handle: string }) => ({
          select: () => ({
            single: async () => {
              const row: ProfileRow = {
                id: payload.id,
                handle: payload.handle,
                display_name: payload.display_name ?? null,
                link_battle_wins: payload.link_battle_wins ?? 0,
                link_battle_losses: payload.link_battle_losses ?? 0,
                link_battle_rating: payload.link_battle_rating ?? 1000,
                total_trades: payload.total_trades ?? 0,
              };
              state.profiles.set(row.id, row);
              return { data: row, error: null };
            },
          }),
        }),
        update: (patch: Partial<ProfileRow>) => ({
          eq: async (_column: string, id: string) => {
            const row = state.profiles.get(id);
            if (row) {
              Object.assign(row, patch);
            }
            return { error: null };
          },
        }),
      };
    }

    if (table === "matches") {
      return {
        select: () => ({
          eq: (_column: string, channelName: string) => ({
            maybeSingle: async () => ({
              data: state.matches.find((match) => match.channel_name === channelName) ?? null,
              error: null,
            }),
          }),
        }),
        insert: (payload: Omit<MatchRow, "id" | "created_at">) => ({
          select: () => ({
            single: async () => {
              const now = new Date().toISOString();
              const row: MatchRow = {
                id: `match-${state.matches.length + 1}`,
                created_at: now,
                ...payload,
              };
              state.matches.push(row);
              return { data: row, error: null };
            },
          }),
        }),
        update: (patch: Partial<MatchRow>) => ({
          eq: (_column: string, id: string) => ({
            select: () => ({
              single: async () => {
                const row = state.matches.find((match) => match.id === id);
                if (row) {
                  Object.assign(row, patch);
                }
                return { data: row ?? null, error: null };
              },
            }),
          }),
        }),
      };
    }

    if (table === "multiplayer_leaderboard") {
      return {
        select: () => ({
          order: () => ({
            limit: async (limit: number) => ({
              data: state.leaderboard.slice(0, limit),
              error: null,
            }),
          }),
        }),
      };
    }

    throw new Error(`Unexpected table ${table}`);
  });

  const rpc = jest.fn(async (_name: string, args: Record<string, any>) => {
    let match = state.matches.find((entry) => entry.channel_name === args.report_channel_name);
    if (!match) {
      match = {
        id: `match-${state.matches.length + 1}`,
        player1_id: args.report_user_id,
        player2_id: args.report_peer_user_id,
        mode: args.report_mode,
        modpack_id: args.report_modpack_id,
        ranked: false,
        status: "active",
        channel_name: args.report_channel_name,
        result: null,
        created_at: new Date().toISOString(),
        started_at: new Date().toISOString(),
        completed_at: null,
      };
      state.matches.push(match);
    }
    if (match.ranked && match.mode === "battle") {
      const local = state.profiles.get(args.report_user_id)!;
      const peer = state.profiles.get(args.report_peer_user_id)!;
      local.link_battle_wins += 1;
      local.link_battle_rating += 16;
      peer.link_battle_losses += 1;
      peer.link_battle_rating -= 16;
    }
    match.result = { ranked: match.ranked };
    return { data: { settled: false, awaitingPeer: true, ranked: match.ranked, match }, error: null };
  });

  return { client: { from, rpc }, state, rpc };
};

const loadRoute = async (serviceClient: unknown) => {
  jest.resetModules();
  jest.doMock("@/lib/supabase/env", () => ({
    isSupabaseConfigured: mockIsSupabaseConfigured,
    isSupabaseServiceRoleConfigured: mockIsSupabaseServiceRoleConfigured,
  }));
  jest.doMock("@/lib/supabase/server", () => ({
    createSupabaseServerClient: () => ({
      auth: { getUser: mockGetUser },
    }),
    createSupabaseServiceRoleClient: () => serviceClient,
  }));
  return await import("./route");
};

describe("multiplayer matches API", () => {
  beforeEach(() => {
    mockIsSupabaseConfigured.mockReset();
    mockIsSupabaseServiceRoleConfigured.mockReset();
    mockGetUser.mockReset();
    mockIsSupabaseConfigured.mockReturnValue(true);
    mockIsSupabaseServiceRoleConfigured.mockReturnValue(true);
    mockGetUser.mockResolvedValue({ data: { user: { id: "local-1" } } });
  });

  it("rejects unauthenticated completion writes", async () => {
    mockGetUser.mockResolvedValue({ data: { user: null } });
    const supabase = buildServiceClient();
    const { POST } = await loadRoute(supabase.client);

    const response = await POST(
      new Request("http://localhost/api/multiplayer/matches", {
        method: "POST",
        body: JSON.stringify({
          channelName: "match-1",
          peerUserId: "peer-1",
          mode: "battle",
          outcome: "local",
        }),
      })
    );

    expect(response.status).toBe(401);
  });

  it("submits a direct battle report as unranked without allowing client-created rating changes", async () => {
    const supabase = buildServiceClient();
    const { POST } = await loadRoute(supabase.client);

    const response = await POST(
      new Request("http://localhost/api/multiplayer/matches", {
        method: "POST",
        body: JSON.stringify({
          channelName: "match-1",
          peerUserId: "peer-1",
          mode: "battle",
          outcome: "local",
        }),
      })
    );

    expect(response.status).toBe(200);
    expect(supabase.state.matches).toHaveLength(1);
    expect(supabase.state.matches[0].status).toBe("active");
    expect(supabase.state.matches[0].modpack_id).toBe("core-modular");
    expect(supabase.state.profiles.get("local-1")?.link_battle_wins).toBe(0);
    expect(supabase.state.profiles.get("peer-1")?.link_battle_losses).toBe(0);
    expect(supabase.state.matches[0].result).toMatchObject({ ranked: false });
  });

  it("updates ratings for a server-created match after validating participants", async () => {
    const supabase = buildServiceClient();
    supabase.state.matches.push({
      id: "ranked-1",
      player1_id: "local-1",
      player2_id: "peer-1",
      mode: "battle",
      modpack_id: "core-modular",
      ranked: true,
      status: "waiting",
      channel_name: "ranked-channel",
      result: null,
      created_at: new Date().toISOString(),
      started_at: null,
      completed_at: null,
    });
    const { POST } = await loadRoute(supabase.client);

    const response = await POST(new Request("http://localhost/api/multiplayer/matches", {
      method: "POST",
      body: JSON.stringify({
        channelName: "ranked-channel",
        peerUserId: "peer-1",
        mode: "battle",
        modpackId: "core-modular",
        outcome: "local",
      }),
    }));

    expect(response.status).toBe(200);
    expect(supabase.state.profiles.get("local-1")?.link_battle_wins).toBe(1);
    expect(supabase.state.profiles.get("peer-1")?.link_battle_losses).toBe(1);
    expect(supabase.state.matches[0].result).toMatchObject({ ranked: true });
  });

  it("does not increment trade counters for client-created direct sessions", async () => {
    const supabase = buildServiceClient();
    const { POST } = await loadRoute(supabase.client);

    const response = await POST(
      new Request("http://localhost/api/multiplayer/matches", {
        method: "POST",
        body: JSON.stringify({
          channelName: "trade-1",
          peerUserId: "peer-1",
          mode: "trade",
          outcome: "draw",
        }),
      })
    );

    expect(response.status).toBe(200);
    expect(supabase.state.profiles.get("local-1")?.total_trades).toBe(0);
    expect(supabase.state.profiles.get("peer-1")?.total_trades).toBe(0);
  });

  it("reads the multiplayer leaderboard", async () => {
    const supabase = buildServiceClient();
    supabase.state.leaderboard.push({ id: "local-1", rank: 1, handle: "trainer" });
    const { GET } = await loadRoute(supabase.client);

    const response = await GET(new Request("http://localhost/api/multiplayer/matches?limit=3"));
    const body = await response.json();

    expect(response.status).toBe(200);
    expect(body.leaderboard).toEqual([{ id: "local-1", rank: 1, handle: "trainer" }]);
  });
});
