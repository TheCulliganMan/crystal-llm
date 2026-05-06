import type { ArenaRun } from "@/arena/types";
const mockCreateSupabaseServiceRoleClient = jest.fn();

const loadRoute = async () => {
  jest.resetModules();
  jest.doMock("@/lib/supabase/server", () => ({
    createSupabaseServiceRoleClient: mockCreateSupabaseServiceRoleClient,
  }));
  return await import("./route");
};

const buildRun = (overrides: Partial<ArenaRun> = {}): ArenaRun => ({
  id: "run-1",
  agent_id: "agent-1",
  created_by: "user-1",
  status: "running",
  queue: "arena",
  seed: null,
  mcp_session_url: "/api/mcp?session_id=existing-session",
  spectator_frame_url: null,
  started_at: "2026-04-15T00:00:00.000Z",
  finished_at: null,
  frame_count: 88,
  badge_count: 0,
  pokedex_seen: 0,
  pokedex_caught: 0,
  error: null,
  metrics: {
    session_id: "existing-session",
  },
  notes: null,
  created_at: "2026-04-15T00:00:00.000Z",
  updated_at: "2026-04-15T00:05:00.000Z",
  agent: {
    id: "agent-1",
    owner_id: "user-1",
    name: "Existing Agent",
    slug: "existing-agent",
    description: null,
    repo_url: null,
    mcp_endpoint: null,
    runtime: "mcp-http",
    visibility: "public",
    config: {},
    created_at: "2026-04-15T00:00:00.000Z",
    updated_at: "2026-04-15T00:00:00.000Z",
  },
  ...overrides,
});

describe("GET /api/arena/runs", () => {
  beforeEach(() => {
    mockCreateSupabaseServiceRoleClient.mockReset();
  });

  it("returns an empty run list when Supabase is unavailable", async () => {
    mockCreateSupabaseServiceRoleClient.mockReturnValue(null);

    const { GET } = await loadRoute();
    const response = await GET(new Request("http://localhost/api/arena/runs?limit=27"));
    expect(response.status).toBe(200);

    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.runs).toEqual([]);
  });

  it("returns only live Supabase runs", async () => {
    const liveRows = [
      buildRun({
        id: "run-existing",
        updated_at: "2026-04-15T00:02:00.000Z",
      }),
    ];
    const query = {
      select: jest.fn().mockReturnThis(),
      in: jest.fn().mockReturnThis(),
      order: jest.fn().mockReturnThis(),
      limit: jest.fn().mockResolvedValue({
        data: liveRows,
        error: null,
      }),
    };
    const supabase = {
      from: jest.fn().mockReturnValue(query),
    };
    mockCreateSupabaseServiceRoleClient.mockReturnValue(supabase);

    const { GET } = await loadRoute();
    const response = await GET(new Request("http://localhost/api/arena/runs?limit=27"));
    expect(response.status).toBe(200);

    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(payload.runs).toHaveLength(1);
    expect(payload.runs[0]).toMatchObject({
      id: "run-existing",
      created_by: "user-1",
    });
  });

  it("clamps oversized watch limits without changing the response shape", async () => {
    const query = {
      select: jest.fn().mockReturnThis(),
      in: jest.fn().mockReturnThis(),
      order: jest.fn().mockReturnThis(),
      limit: jest.fn().mockResolvedValue({
        data: [],
        error: null,
      }),
    };
    const supabase = {
      from: jest.fn().mockReturnValue(query),
    };
    mockCreateSupabaseServiceRoleClient.mockReturnValue(supabase);

    const { GET } = await loadRoute();
    const response = await GET(new Request("http://localhost/api/arena/runs?limit=999"));
    expect(response.status).toBe(200);
    const payload = await response.json();
    expect(payload.ok).toBe(true);
    expect(query.limit).toHaveBeenCalledWith(27);
  });
});
