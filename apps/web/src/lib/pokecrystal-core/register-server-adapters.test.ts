const mockCreateSupabaseServiceRoleClient = jest.fn();

jest.mock("@/lib/supabase/server", () => ({
  createSupabaseServiceRoleClient: mockCreateSupabaseServiceRoleClient,
}));

describe("register-server-adapters", () => {
  beforeEach(() => {
    jest.resetModules();
    mockCreateSupabaseServiceRoleClient.mockReset();
  });

  it("treats identity cloud save as optional when Supabase is unavailable", async () => {
    await jest.isolateModulesAsync(async () => {
      const adapters = await import("@pokecrystal/core/adapters");
      adapters.resetCloudSaveAdapters();
      mockCreateSupabaseServiceRoleClient.mockReturnValue(null);

      await import("./register-server-adapters");

      await expect(adapters.loadIdentityCloudSave("slot-1", "player-1")).resolves.toBeNull();
      await expect(
        adapters.saveIdentityCloudSave("slot-1", "player-1", { progress: "new-bark-town" }),
      ).resolves.toBeUndefined();
      await expect(adapters.deleteIdentityCloudSave("slot-1", "player-1")).resolves.toBe(false);

      adapters.resetCloudSaveAdapters();
    });
  });
});
