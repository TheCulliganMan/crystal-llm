const mockCreateSupabaseServiceRoleClient = jest.fn();

jest.mock("@/lib/supabase/server", () => ({
  createSupabaseServiceRoleClient: (...args: unknown[]) => mockCreateSupabaseServiceRoleClient(...args),
}));

const makeMissingTableClient = () => {
  const arenaRunsQuery = {
    select: jest.fn().mockReturnThis(),
    in: jest.fn().mockReturnThis(),
    order: jest.fn().mockReturnThis(),
    limit: jest.fn().mockResolvedValue({
      data: null,
      error: { code: "PGRST205", message: "Could not find relation arena_runs" },
    }),
  };
  return {
    from: jest.fn((table: string) => {
      if (table === "arena_runs") {
        return arenaRunsQuery;
      }
      throw new Error(`Unexpected table: ${table}`);
    }),
  };
};

describe("arena telemetry missing-schema handling", () => {
  beforeEach(() => {
    jest.resetModules();
    mockCreateSupabaseServiceRoleClient.mockReset();
  });

  it("disables telemetry on PGRST205 and logs debug once", async () => {
    const debugSpy = jest.spyOn(console, "debug").mockImplementation(() => {});
    const warnSpy = jest.spyOn(console, "warn").mockImplementation(() => {});
    const client = makeMissingTableClient();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);

    const telemetry = await import("./telemetry");
    telemetry.__testing.resetTelemetryState();

    await telemetry.ensureArenaRunForSession("session-1");
    await telemetry.ensureArenaRunForSession("session-2");

    expect(telemetry.__testing.isTelemetryDisabled()).toBe(true);
    expect(mockCreateSupabaseServiceRoleClient).toHaveBeenCalledTimes(1);
    expect(debugSpy).toHaveBeenCalledTimes(1);
    expect(warnSpy).not.toHaveBeenCalled();

    debugSpy.mockRestore();
    warnSpy.mockRestore();
  });
});

const makeDuplicateInsertClient = () => {
  const recoveredRun = {
    id: "run-123",
    status: "running",
    metrics: { session_id: "session-1" },
    mcp_session_url: "/api/mcp?session_id=session-1",
  };
  const arenaRunsQuery = {
    select: jest.fn().mockReturnThis(),
    in: jest.fn().mockReturnThis(),
    order: jest.fn().mockReturnThis(),
    limit: jest.fn()
      .mockResolvedValueOnce({ data: [], error: null })
      .mockResolvedValueOnce({ data: [recoveredRun], error: null }),
    insert: jest.fn().mockReturnThis(),
    maybeSingle: jest.fn().mockResolvedValue({
      data: null,
      error: {
        code: "23505",
        message: 'duplicate key value violates unique constraint "uq_arena_runs_active_agent_session"',
      },
    }),
  };
  const arenaAgentsQuery = {
    select: jest.fn().mockReturnThis(),
    eq: jest.fn().mockReturnThis(),
    maybeSingle: jest.fn().mockResolvedValue({ data: { id: "agent-123" }, error: null }),
  };
  return {
    from: jest.fn((table: string) => {
      if (table === "arena_runs") {
        return arenaRunsQuery;
      }
      if (table === "arena_agents") {
        return arenaAgentsQuery;
      }
      throw new Error(`Unexpected table: ${table}`);
    }),
    auth: {
      admin: {
        listUsers: jest.fn().mockResolvedValue({
          data: { users: [{ id: "user-123", email: "mcp-session@pokecrystal.local" }] },
          error: null,
        }),
      },
    },
  };
};

describe("arena telemetry duplicate-run handling", () => {
  beforeEach(() => {
    jest.resetModules();
    mockCreateSupabaseServiceRoleClient.mockReset();
  });

  it("recovers by refetching the active run when insert hits the unique constraint", async () => {
    const warnSpy = jest.spyOn(console, "warn").mockImplementation(() => {});
    const client = makeDuplicateInsertClient();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);

    const telemetry = await import("./telemetry");
    telemetry.__testing.resetTelemetryState();

    const result = await telemetry.ensureArenaRunForSession("session-1");

    expect(result).toMatchObject({ id: "run-123", status: "running" });
    expect(warnSpy).not.toHaveBeenCalledWith(
      expect.stringContaining("[arena-telemetry] failed to insert MCP run"),
      expect.anything()
    );

    warnSpy.mockRestore();
  });

  it("scans a wider active-run window before falling back to insert", async () => {
    const warnSpy = jest.spyOn(console, "warn").mockImplementation(() => {});
    const client = makeDuplicateInsertClient();
    mockCreateSupabaseServiceRoleClient.mockReturnValue(client);

    const telemetry = await import("./telemetry");
    telemetry.__testing.resetTelemetryState();

    await telemetry.ensureArenaRunForSession("session-1");

    const arenaRunsQuery = client.from("arena_runs");
    expect(arenaRunsQuery.limit).toHaveBeenCalledWith(200);
    expect(warnSpy).not.toHaveBeenCalledWith(
      expect.stringContaining("[arena-telemetry] failed to insert MCP run"),
      expect.anything()
    );

    warnSpy.mockRestore();
  });
});
