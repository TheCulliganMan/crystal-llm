const mockIsRequestAuthorized = jest.fn();
const mockCreateSupabaseServiceRoleClient = jest.fn();
const mockVerifySessionSecret = jest.fn();
const mockHasValidIdentityToken = jest.fn();

const SESSION_ID_REGEX = /^[a-zA-Z0-9_-]{1,64}$/;

const buildSupabase = () => {
  const upsertSelectMaybeSingle = jest.fn();
  const upsert = jest.fn(() => ({
    select: jest.fn(() => ({
      maybeSingle: upsertSelectMaybeSingle,
    })),
  }));
  const insertSelectMaybeSingle = jest.fn();
  const insert = jest.fn(() => ({
    select: jest.fn(() => ({
      maybeSingle: insertSelectMaybeSingle,
    })),
  }));
  const runEventInsert = jest.fn().mockResolvedValue({ error: null });
  const updatePayload = jest.fn();
  const updateEq = jest.fn();
  const update = jest.fn((payload: unknown) => {
    updatePayload(payload);
    return {
    eq: updateEq,
    };
  });
  const containsOrderLimitMaybeSingle = jest.fn();
  const contains = jest.fn(() => ({
    order: jest.fn(() => ({
      limit: jest.fn(() => ({
        maybeSingle: containsOrderLimitMaybeSingle,
      })),
    })),
  }));
  const ownerMaybeSingle = jest.fn();
  const limit = jest.fn(() => ({
    maybeSingle: ownerMaybeSingle,
  }));
  const from = jest.fn((table: string) => {
    if (table === "arena_agents") {
      return {
        upsert,
        select: jest.fn(() => ({ limit })),
      };
    }
    if (table === "arena_runs") {
      return {
        select: jest.fn(() => ({ eq: jest.fn(() => ({ contains })) })),
        insert,
        update,
      };
    }
    if (table === "arena_run_events") {
      return {
        insert: runEventInsert,
      };
    }
    throw new Error(`Unexpected table ${table}`);
  });

  return {
    client: { from },
    mocks: {
      upsertSelectMaybeSingle,
      containsOrderLimitMaybeSingle,
      insertSelectMaybeSingle,
      updatePayload,
      updateEq,
      ownerMaybeSingle,
      runEventInsert,
      from,
    },
  };
};

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/app/mcp/session-guards", () => ({
    isRequestAuthorized: mockIsRequestAuthorized,
    SESSION_ID_REGEX,
    verifySessionSecret: mockVerifySessionSecret,
  }));
  jest.doMock("@/app/api/[transport]/tools/identity", () => ({
    hasValidIdentityToken: mockHasValidIdentityToken,
  }));
  jest.doMock("@/lib/supabase/server", () => ({
    createSupabaseServiceRoleClient: mockCreateSupabaseServiceRoleClient,
  }));
  return await import("./route");
};

describe("arena progress ingest API", () => {
  beforeEach(() => {
    mockIsRequestAuthorized.mockReset();
    mockCreateSupabaseServiceRoleClient.mockReset();
    mockVerifySessionSecret.mockReset();
    mockHasValidIdentityToken.mockReset();
    mockIsRequestAuthorized.mockReturnValue(true);
    mockVerifySessionSecret.mockReturnValue({ ok: true, status: 200, playerId: "player-1" });
    mockHasValidIdentityToken.mockReturnValue(false);
    process.env.POKECRYSTAL_MCP_SYSTEM_USER_ID = "11111111-1111-4111-8111-111111111111";
    process.env.POKECRYSTAL_ARENA_PROGRESS_TOKEN = "";
  });

  it("creates run progress for a public agent", async () => {
    const { client, mocks } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);
    mocks.upsertSelectMaybeSingle.mockResolvedValue({
      data: { id: "agent-1", name: "Agent Prime" },
      error: null,
    });
    mocks.containsOrderLimitMaybeSingle.mockResolvedValue({
      data: null,
      error: null,
    });
    mocks.insertSelectMaybeSingle.mockResolvedValue({
      data: { id: "run-1", status: "running" },
      error: null,
    });

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/progress", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          sessionId: "agent-session-1",
          agentName: "Agent Prime",
          stepCount: 120,
          instructionCount: 80,
          frameCount: 500,
        }),
      })
    );

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.created).toBe(true);
    expect(payload.runId).toBe("run-1");
    expect(mocks.runEventInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        run_id: "run-1",
        frame: 500,
        label: "progress_update",
        payload: expect.objectContaining({
          session_id: "agent-session-1",
          created: true,
          step_count: 120,
          instruction_count: 80,
        }),
      })
    );
  });

  it("updates existing run progress and preserves monotonic counters", async () => {
    const { client, mocks } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);
    mocks.upsertSelectMaybeSingle.mockResolvedValue({
      data: { id: "agent-2", name: "Agent Delta" },
      error: null,
    });
    mocks.containsOrderLimitMaybeSingle.mockResolvedValue({
      data: {
        id: "run-existing",
        queue: "main",
        status: "running",
        started_at: "2026-02-18T10:00:00.000Z",
        finished_at: null,
        frame_count: 1000,
        badge_count: 1,
        pokedex_seen: 12,
        pokedex_caught: 3,
        error: null,
        notes: null,
        metrics: { step_count: 200, command_count: 50, session_id: "agent-session-2" },
      },
      error: null,
    });
    mocks.updateEq.mockResolvedValue({ error: null });

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/progress", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          sessionId: "agent-session-2",
          agentName: "Agent Delta",
          stepCount: 190,
          instructionCount: 55,
        }),
      })
    );

    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.created).toBe(false);
    const updateArg = mocks.updatePayload.mock.calls[0][0] as {
      metrics: { step_count: number; command_count: number };
    };
    expect(updateArg.metrics.step_count).toBe(200);
    expect(updateArg.metrics.command_count).toBe(55);
    expect(mocks.runEventInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        run_id: "run-existing",
        label: "progress_update",
        payload: expect.objectContaining({
          session_id: "agent-session-2",
          created: false,
          step_count: 190,
          instruction_count: 55,
        }),
      })
    );
  });

  it("stores optional flow-state progress metadata without changing core counters", async () => {
    const { client, mocks } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);
    mocks.upsertSelectMaybeSingle.mockResolvedValue({
      data: { id: "agent-flow", name: "Flow Agent" },
      error: null,
    });
    mocks.containsOrderLimitMaybeSingle.mockResolvedValue({
      data: null,
      error: null,
    });
    mocks.insertSelectMaybeSingle.mockResolvedValue({
      data: { id: "run-flow", status: "running" },
      error: null,
    });

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/progress", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          sessionId: "agent-session-flow",
          agentName: "Flow Agent",
          flowState: {
            summary: "Next goal: Zephyr Badge",
            nextGoal: "Zephyr Badge",
            completionTarget: "Beat Mt. Silver",
            completedIds: ["starter", "mr-pokemon"],
          },
        }),
      })
    );

    expect(response.status).toBe(200);
    expect(mocks.runEventInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        payload: expect.objectContaining({
          flow_state: expect.objectContaining({
            summary: "Next goal: Zephyr Badge",
            nextGoal: "Zephyr Badge",
            completionTarget: "Beat Mt. Silver",
            completedIds: ["starter", "mr-pokemon"],
          }),
        }),
      })
    );
  });

  it("rejects updates when an existing run is bound to a different identity", async () => {
    const { client, mocks } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);
    mocks.upsertSelectMaybeSingle.mockResolvedValue({
      data: { id: "agent-3", name: "Agent Sigma" },
      error: null,
    });
    mocks.containsOrderLimitMaybeSingle.mockResolvedValue({
      data: {
        id: "run-owned",
        queue: "main",
        status: "running",
        started_at: "2026-02-18T10:00:00.000Z",
        finished_at: null,
        frame_count: 1000,
        badge_count: 1,
        pokedex_seen: 12,
        pokedex_caught: 3,
        error: null,
        notes: null,
        metrics: { session_id: "agent-session-3", owner_player_id: "player-2" },
      },
      error: null,
    });

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/progress", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          sessionId: "agent-session-3",
          agentName: "Agent Sigma",
          stepCount: 220,
        }),
      })
    );

    expect(response.status).toBe(403);
    const payload = await response.json();
    expect(payload.ok).toBe(false);
    expect(payload.error).toContain("bound");
  });

  it("rejects invalid payloads and unauthorized requests", async () => {
    const { POST } = await loadRoute();
    process.env.POKECRYSTAL_ARENA_PROGRESS_TOKEN = "progress-token";
    mockIsRequestAuthorized.mockReturnValue(false);
    const unauthorized = await POST(
      new Request("http://localhost/api/arena/progress", { method: "POST", body: "{}" })
    );
    expect(unauthorized.status).toBe(401);
    process.env.POKECRYSTAL_ARENA_PROGRESS_TOKEN = "";

    mockIsRequestAuthorized.mockReturnValue(true);
    const badRequest = await POST(
      new Request("http://localhost/api/arena/progress", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ sessionId: "bad id", agentName: "" }),
      })
    );
    expect(badRequest.status).toBe(400);
  });

  it("accepts identity-token auth for token-protected progress updates", async () => {
    process.env.POKECRYSTAL_ARENA_PROGRESS_TOKEN = "progress-token";
    mockIsRequestAuthorized.mockReturnValue(false);
    mockHasValidIdentityToken.mockReturnValue(true);

    const { client, mocks } = buildSupabase();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);
    mocks.upsertSelectMaybeSingle.mockResolvedValue({
      data: { id: "agent-identity", name: "Identity Agent" },
      error: null,
    });
    mocks.containsOrderLimitMaybeSingle.mockResolvedValue({
      data: null,
      error: null,
    });
    mocks.insertSelectMaybeSingle.mockResolvedValue({
      data: { id: "run-identity", status: "running" },
      error: null,
    });

    const { POST } = await loadRoute();
    const response = await POST(
      new Request("http://localhost/api/arena/progress", {
        method: "POST",
        headers: { "content-type": "application/json", authorization: "Bearer token" },
        body: JSON.stringify({
          sessionId: "agent-session-identity",
          agentName: "Identity Agent",
        }),
      })
    );

    expect(response.status).toBe(200);
    expect(mockVerifySessionSecret).toHaveBeenCalledWith(expect.any(Request), "agent-session-identity");
    process.env.POKECRYSTAL_ARENA_PROGRESS_TOKEN = "";
  });
});
