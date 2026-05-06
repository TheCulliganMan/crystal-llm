export {};

const originalEnv = process.env;

describe("arena queries without supabase config", () => {
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

  it("returns empty defaults instead of throwing", async () => {
    const queries = await import("./queries");
    await expect(queries.getCurrentUser()).resolves.toBeNull();
    await expect(queries.fetchProfile("user-1")).resolves.toBeNull();
    await expect(queries.fetchOwnedAgents("user-1")).resolves.toEqual([]);
    await expect(queries.fetchRecentRuns()).resolves.toEqual([]);
    await expect(queries.fetchActiveRuns()).resolves.toEqual([]);
    await expect(queries.fetchRunById("run-1")).resolves.toBeNull();
    await expect(queries.fetchLeaderboard()).resolves.toEqual({ leaderboard: [], agents: [] });
    await expect(queries.fetchPublicAgents()).resolves.toEqual([]);
  });
});
