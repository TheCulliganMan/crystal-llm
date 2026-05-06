const mockCreateSupabaseServiceRoleClient = jest.fn();
const mockIsRequestAuthorized = jest.fn();
const mockHasValidIdentityToken = jest.fn();
const mockVerifySessionSecret = jest.fn();

const FIXED_OWNER = "11111111-1111-4111-8111-111111111111";

type AgentRow = {
  id: string;
  owner_id: string;
  name: string;
  slug: string;
  runtime: string;
  visibility: "public";
  repo_url: string | null;
  config: Record<string, unknown>;
};

type MatchRow = {
  id: string;
  challenger_agent_id: string;
  opponent_agent_id: string;
  created_by: string;
  queue: string;
  status: "pending" | "running" | "completed" | "cancelled";
  outcome: "challenger" | "opponent" | "draw" | "cancelled" | null;
  winner_agent_id: string | null;
  challenger_session_id: string | null;
  opponent_session_id: string | null;
  challenger_score: number | null;
  opponent_score: number | null;
  notes: string | null;
  metadata: Record<string, unknown>;
  started_at: string;
  finished_at: string | null;
  created_at: string;
  updated_at: string;
};

type RatingRow = {
  agent_id: string;
  rating: number;
  games_played: number;
  wins: number;
  losses: number;
  draws: number;
  last_match_at: string | null;
  created_at: string;
  updated_at: string;
};

const buildSupabase = () => {
  const state = {
    agents: [] as AgentRow[],
    matches: [] as MatchRow[],
    ratings: [] as RatingRow[],
    leaderboard: [] as Array<Record<string, unknown>>,
  };

  let agentCounter = 0;
  let matchCounter = 0;

  const upsertAgent = (payload: Partial<AgentRow> & { owner_id: string; name: string; slug: string }) => {
    const existing = state.agents.find((agent) => agent.owner_id === payload.owner_id && agent.name === payload.name);
    if (existing) {
      Object.assign(existing, payload);
      return existing;
    }
    const created: AgentRow = {
      id: `agent-${++agentCounter}`,
      owner_id: payload.owner_id,
      name: payload.name,
      slug: payload.slug,
      runtime: payload.runtime ?? "mcp-http",
      visibility: "public",
      repo_url: payload.repo_url ?? null,
      config: (payload.config as Record<string, unknown>) ?? {},
    };
    state.agents.push(created);
    return created;
  };

  const insertMatch = (payload: Record<string, unknown>) => {
    const now = new Date().toISOString();
    const created: MatchRow = {
      id: `match-${++matchCounter}`,
      challenger_agent_id: String(payload.challenger_agent_id),
      opponent_agent_id: String(payload.opponent_agent_id),
      created_by: String(payload.created_by),
      queue: String(payload.queue ?? "krabbyclaw-arena"),
      status: (payload.status as MatchRow["status"]) ?? "running",
      outcome: (payload.outcome as MatchRow["outcome"]) ?? null,
      winner_agent_id: (payload.winner_agent_id as string | null) ?? null,
      challenger_session_id: (payload.challenger_session_id as string | null) ?? null,
      opponent_session_id: (payload.opponent_session_id as string | null) ?? null,
      challenger_score: (payload.challenger_score as number | null) ?? null,
      opponent_score: (payload.opponent_score as number | null) ?? null,
      notes: (payload.notes as string | null) ?? null,
      metadata: (payload.metadata as Record<string, unknown>) ?? {},
      started_at: String(payload.started_at ?? now),
      finished_at: (payload.finished_at as string | null) ?? null,
      created_at: now,
      updated_at: String(payload.updated_at ?? now),
    };
    state.matches.unshift(created);
    return created;
  };

  const upsertRating = (payload: Partial<RatingRow> & { agent_id: string }) => {
    const existing = state.ratings.find((rating) => rating.agent_id === payload.agent_id);
    if (existing) {
      Object.assign(existing, payload);
      return existing;
    }
    const now = new Date().toISOString();
    const created: RatingRow = {
      agent_id: payload.agent_id,
      rating: payload.rating ?? 1000,
      games_played: payload.games_played ?? 0,
      wins: payload.wins ?? 0,
      losses: payload.losses ?? 0,
      draws: payload.draws ?? 0,
      last_match_at: payload.last_match_at ?? null,
      created_at: now,
      updated_at: payload.updated_at ?? now,
    };
    state.ratings.push(created);
    return created;
  };

  const from = jest.fn((table: string) => {
    if (table === "arena_agents") {
      return {
        upsert: (payload: Record<string, unknown>) => {
          const row = upsertAgent(payload as Partial<AgentRow> & { owner_id: string; name: string; slug: string });
          return {
            select: () => ({
              maybeSingle: async () => ({
                data: { id: row.id, name: row.name, slug: row.slug, runtime: row.runtime },
                error: null,
              }),
            }),
          };
        },
        select: () => ({
          in: async (_column: string, ids: string[]) => ({
            data: state.agents
              .filter((agent) => ids.includes(agent.id))
              .map((agent) => ({ id: agent.id, name: agent.name, slug: agent.slug, runtime: agent.runtime })),
            error: null,
          }),
          limit: () => ({
            maybeSingle: async () => ({
              data: state.agents[0] ? { owner_id: state.agents[0].owner_id } : null,
              error: null,
            }),
          }),
        }),
      };
    }

    if (table === "krabbyclaw_arena_matches") {
      return {
        insert: (payload: Record<string, unknown>) => {
          const row = insertMatch(payload);
          return {
            select: () => ({
              maybeSingle: async () => ({ data: row, error: null }),
            }),
          };
        },
        select: () => ({
          eq: (_column: string, value: string) => ({
            maybeSingle: async () => ({
              data: state.matches.find((match) => match.id === value) ?? null,
              error: null,
            }),
            order: (_orderColumn: string, _options: unknown) => ({
              limit: async (limit: number) => ({
                data: state.matches
                  .filter((match) => match.status === value)
                  .slice(0, limit),
                error: null,
              }),
            }),
          }),
          order: (_column: string, _options: unknown) => ({
            limit: async (limit: number) => ({ data: state.matches.slice(0, limit), error: null }),
          }),
        }),
        update: (payload: Record<string, unknown>) => ({
          eq: async (_column: string, value: string) => {
            const row = state.matches.find((match) => match.id === value);
            if (row) {
              Object.assign(row, payload);
            }
            return { error: null };
          },
        }),
      };
    }

    if (table === "krabbyclaw_arena_ratings") {
      return {
        select: () => ({
          eq: (_column: string, value: string) => ({
            maybeSingle: async () => ({ data: state.ratings.find((rating) => rating.agent_id === value) ?? null, error: null }),
          }),
        }),
        upsert: (payload: Record<string, unknown>) => {
          const row = upsertRating(payload as Partial<RatingRow> & { agent_id: string });
          return {
            select: () => ({
              maybeSingle: async () => ({ data: row, error: null }),
            }),
          };
        },
        update: (payload: Record<string, unknown>) => ({
          eq: async (_column: string, value: string) => {
            const row = state.ratings.find((rating) => rating.agent_id === value);
            if (row) {
              Object.assign(row, payload);
            }
            return { error: null };
          },
        }),
      };
    }

    if (table === "krabbyclaw_arena_leaderboard") {
      return {
        select: () => ({
          order: (_column: string, _options: unknown) => ({
            limit: async (limit: number) => ({ data: state.leaderboard.slice(0, limit), error: null }),
          }),
        }),
      };
    }

    if (table === "arena_runs") {
      return {
        select: () => ({
          contains: () => ({
            order: () => ({
              limit: () => ({
                maybeSingle: async () => ({ data: null, error: null }),
              }),
            }),
          }),
        }),
      };
    }

    throw new Error(`Unexpected table ${table}`);
  });

  return {
    client: { from },
    state,
  };
};

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/lib/supabase/server", () => ({
    createSupabaseServiceRoleClient: mockCreateSupabaseServiceRoleClient,
  }));
  jest.doMock("@/app/mcp/session-guards", () => ({
    isRequestAuthorized: mockIsRequestAuthorized,
    SESSION_ID_REGEX: /^[a-zA-Z0-9_-]{1,64}$/,
    verifySessionSecret: mockVerifySessionSecret,
  }));
  jest.doMock("@/app/api/[transport]/tools/identity", () => ({
    hasValidIdentityToken: mockHasValidIdentityToken,
  }));
  return await import("./route");
};

describe("KrabbyClawArena API route", () => {
  beforeEach(() => {
    mockCreateSupabaseServiceRoleClient.mockReset();
    mockIsRequestAuthorized.mockReset();
    mockHasValidIdentityToken.mockReset();
    mockVerifySessionSecret.mockReset();

    process.env.POKECRYSTAL_MCP_SYSTEM_USER_ID = FIXED_OWNER;
    process.env.POKECRYSTAL_ARENA_PROGRESS_TOKEN = "";

    mockIsRequestAuthorized.mockReturnValue(true);
    mockHasValidIdentityToken.mockReturnValue(false);
    mockVerifySessionSecret.mockReturnValue({ ok: true, status: 200 });
  });

  it("returns leaderboard and match snapshots", async () => {
    const { client, state } = buildSupabase();
    state.leaderboard.push({
      rank: 1,
      agent_id: "agent-1",
      agent_name: "Krabby Prime",
      rating: 1042,
      games_played: 10,
      wins: 6,
      losses: 3,
      draws: 1,
      win_rate: 60,
    });
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);

    const { GET } = await loadRoute();
    const response = await GET(new Request("http://localhost/api/arena/krabbyclaw?limit=5"));

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.leaderboard).toHaveLength(1);
    expect(payload.activeMatches).toHaveLength(0);
    expect(payload.queue).toHaveLength(0);
  });

  it("queues and matches agents", async () => {
    const { client, state } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);

    const { POST } = await loadRoute();
    const queued = await POST(
      new Request("http://localhost/api/arena/krabbyclaw", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action: "queue",
          agent: {
            name: "Krabby Prime",
            sessionId: "krabby-prime-1",
            team: ["Pikachu Lv50", "Gengar Lv50"],
          },
        }),
      }),
    );

    expect(queued.status).toBe(200);
    const queuedPayload = await queued.json();
    expect(queuedPayload.ok).toBe(true);
    expect(queuedPayload.matched).toBe(false);
    expect(state.matches[0]?.status).toBe("pending");

    const matched = await POST(
      new Request("http://localhost/api/arena/krabbyclaw", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action: "queue",
          agent: {
            name: "Kingler Core",
            sessionId: "kingler-core-1",
            team: ["Starmie Lv50", "Snorlax Lv50"],
          },
        }),
      }),
    );

    expect(matched.status).toBe(200);
    const matchedPayload = await matched.json();
    expect(matchedPayload.ok).toBe(true);
    expect(matchedPayload.matched).toBe(true);
    expect(state.matches[0]?.status).toBe("running");
  });

  it("creates and reports a battle with ELO updates", async () => {
    const { client, state } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/krabbyclaw", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action: "report",
          challenger: {
            name: "Krabby Prime",
            sessionId: "krabby-prime-1",
            team: ["Pikachu Lv50", "Gengar Lv50"],
          },
          opponent: {
            name: "Kingler Core",
            sessionId: "kingler-core-1",
            team: ["Starmie Lv50", "Snorlax Lv50"],
          },
          outcome: "challenger",
          challengerScore: 4,
          opponentScore: 2,
        }),
      }),
    );

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.outcome).toBe("challenger");
    expect(state.matches[0]?.status).toBe("completed");
    expect(state.ratings).toHaveLength(2);

    const challengerRating = state.ratings.find((rating) => rating.agent_id === state.matches[0].challenger_agent_id);
    const opponentRating = state.ratings.find((rating) => rating.agent_id === state.matches[0].opponent_agent_id);
    expect(challengerRating?.rating).toBeGreaterThan(1000);
    expect(opponentRating?.rating).toBeLessThan(1000);
  });

  it("requires session secret when identity auth is used", async () => {
    const { client } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);
    mockHasValidIdentityToken.mockReturnValue(true);
    mockVerifySessionSecret.mockReturnValue({ ok: false, status: 401, message: "Missing session secret." });

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/krabbyclaw", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action: "queue",
          controllerSessionId: "identity-controller",
          agent: {
            name: "Krabby Prime",
            sessionId: "krabby-prime-1",
            team: ["Pikachu Lv50", "Gengar Lv50"],
          },
        }),
      }),
    );

    expect(response.status).toBe(401);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toContain("Missing session secret");
  });

  it("requires session-secret auth even without static arena tokens", async () => {
    const { client } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);
    mockHasValidIdentityToken.mockReturnValue(false);
    mockVerifySessionSecret.mockReturnValue({ ok: false, status: 401, message: "Missing identity token." });

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/krabbyclaw", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          action: "queue",
          agent: {
            name: "Krabby Prime",
            sessionId: "krabby-prime-1",
            team: ["Pikachu Lv50", "Gengar Lv50"],
          },
        }),
      }),
    );

    expect(response.status).toBe(401);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toContain("Missing identity token");
    expect(mockVerifySessionSecret).toHaveBeenCalledWith(expect.any(Request), "krabby-prime-1");
  });
});
