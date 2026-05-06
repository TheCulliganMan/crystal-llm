import { promises as fs } from "fs";
import path from "path";
import {
  resetCloudSaveAdapters,
  setIdentityCloudSaveAdapter,
} from "@pokecrystal/core/adapters/cloud-save";
import { createInitialGameState } from "../state";
import {
  SaveFileNotFoundError,
  SaveGameError,
  deleteSaveGame,
  hasSaveGame,
  loadGame,
  saveGame,
} from "../save";
import { runWithMcpIdentityContext } from "../mcp-identity-context.server";

type IdentityContext = {
  playerId: string;
  token: string;
  name: string;
};

const IDENTITY: IdentityContext = {
  playerId: "player-identity-test",
  token: "identity-token",
  name: "Identity Tester",
};

const SLOT = "identity-strict-save.sav";
const LOCAL_IDENTITY_SLOT = `${IDENTITY.playerId}__${SLOT}`;
const LOCAL_IDENTITY_PATH = path.resolve(process.cwd(), LOCAL_IDENTITY_SLOT);
const LOCAL_IDENTITY_BACKUP_PATH = `${LOCAL_IDENTITY_PATH}.bak`;
const PAYLOAD_SOURCE_SLOT = "identity-supabase-payload-source.sav";
const PAYLOAD_SOURCE_PATH = path.resolve(process.cwd(), PAYLOAD_SOURCE_SLOT);
const PAYLOAD_SOURCE_BACKUP_PATH = `${PAYLOAD_SOURCE_PATH}.bak`;

const withIdentity = async <T>(fn: () => Promise<T>): Promise<T> =>
  runWithMcpIdentityContext(IDENTITY, fn);

const buildValidSerializedSavePayload = async (): Promise<Record<string, unknown>> => {
  await saveGame(createInitialGameState(), PAYLOAD_SOURCE_SLOT);
  const serialized = await fs.readFile(PAYLOAD_SOURCE_PATH, "utf-8");
  return JSON.parse(serialized) as Record<string, unknown>;
};

const buildIdentityLoadClient = (payload: Record<string, unknown> | null) => {
  const maybeSingle = jest.fn().mockResolvedValue({
    data: payload ? { payload } : null,
    error: null,
  });
  const query = {
    select: jest.fn().mockReturnThis(),
    eq: jest.fn().mockReturnThis(),
    maybeSingle,
  };
  return {
    from: jest.fn().mockReturnValue(query),
    maybeSingle,
  };
};

describe("MCP identity Supabase persistence", () => {
  const originalEnv = {
    url: process.env.NEXT_PUBLIC_SUPABASE_URL,
    serviceRoleKey: process.env.SUPABASE_SERVICE_ROLE_KEY,
  };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_SUPABASE_URL = "https://example.supabase.co";
    process.env.SUPABASE_SERVICE_ROLE_KEY = "service-role-key";
    resetCloudSaveAdapters();
  });

  afterEach(async () => {
    resetCloudSaveAdapters();
    process.env.NEXT_PUBLIC_SUPABASE_URL = originalEnv.url;
    process.env.SUPABASE_SERVICE_ROLE_KEY = originalEnv.serviceRoleKey;
    jest.restoreAllMocks();
    await fs.unlink(LOCAL_IDENTITY_PATH).catch(() => undefined);
    await fs.unlink(LOCAL_IDENTITY_BACKUP_PATH).catch(() => undefined);
    await fs.unlink(PAYLOAD_SOURCE_PATH).catch(() => undefined);
    await fs.unlink(PAYLOAD_SOURCE_BACKUP_PATH).catch(() => undefined);
  });

  it("fails identity saves when Supabase service role access is unavailable", async () => {
    delete process.env.NEXT_PUBLIC_SUPABASE_URL;
    delete process.env.SUPABASE_SERVICE_ROLE_KEY;
    setIdentityCloudSaveAdapter(null);

    await expect(
      withIdentity(async () => saveGame(createInitialGameState(), SLOT))
    ).rejects.toThrow(SaveGameError);
    await expect(fs.readFile(LOCAL_IDENTITY_PATH, "utf-8")).rejects.toThrow();
  });

  it("writes identity saves to Supabase only (no local mirror)", async () => {
    const upsert = jest.fn().mockResolvedValue({ error: null });
    setIdentityCloudSaveAdapter({
      loadForIdentity: async () => null,
      saveForIdentity: async (slot, playerId, snapshot) => {
        await upsert({
          user_id: playerId,
          slot,
          payload: snapshot,
          updated_at: new Date().toISOString(),
        });
      },
      deleteForIdentity: async () => false,
    });

    const saved = await withIdentity(async () =>
      saveGame(createInitialGameState(), SLOT)
    );

    expect(saved).toBe(true);
    expect(upsert).toHaveBeenCalledTimes(1);
    expect(upsert).toHaveBeenCalledWith(
      expect.objectContaining({
        user_id: IDENTITY.playerId,
        slot: SLOT,
        payload: expect.objectContaining({ sram: expect.any(Object) }),
        updated_at: expect.any(String),
      }),
    );
    await expect(fs.readFile(LOCAL_IDENTITY_PATH, "utf-8")).rejects.toThrow();
    await expect(fs.readFile(LOCAL_IDENTITY_BACKUP_PATH, "utf-8")).rejects.toThrow();
  });

  it("does not fall back to local identity files when Supabase slot is missing", async () => {
    await saveGame(createInitialGameState(), LOCAL_IDENTITY_SLOT);
    await expect(fs.readFile(LOCAL_IDENTITY_PATH, "utf-8")).resolves.toContain('"sram"');

    const client = buildIdentityLoadClient(null);
    setIdentityCloudSaveAdapter({
      loadForIdentity: async () => {
        const { data } = await client.maybeSingle();
        return (data?.payload as Record<string, unknown> | null) ?? null;
      },
      saveForIdentity: async () => undefined,
      deleteForIdentity: async () => false,
    });

    await expect(withIdentity(async () => loadGame(SLOT))).rejects.toThrow(
      SaveFileNotFoundError
    );
    expect(client.maybeSingle).toHaveBeenCalledTimes(1);
  });

  it("loads identity saves from Supabase payloads", async () => {
    const payload = await buildValidSerializedSavePayload();
    const sram = payload["sram"] as Record<string, unknown>;
    sram["player_name"] = "SupabaseOnly";
    const client = buildIdentityLoadClient(payload);
    setIdentityCloudSaveAdapter({
      loadForIdentity: async () => {
        const { data } = await client.maybeSingle();
        return (data?.payload as Record<string, unknown> | null) ?? null;
      },
      saveForIdentity: async () => undefined,
      deleteForIdentity: async () => false,
    });

    const loaded = await withIdentity(async () => loadGame(SLOT));

    expect(loaded.sram.player_name).toBe("SupabaseOnly");
    expect(client.maybeSingle).toHaveBeenCalledTimes(1);
  });

  it("deletes identity saves through the identity adapter without touching local scoped files", async () => {
    await saveGame(createInitialGameState(), LOCAL_IDENTITY_SLOT);
    await expect(fs.readFile(LOCAL_IDENTITY_PATH, "utf-8")).resolves.toContain('"sram"');
    const deleteForIdentity = jest.fn().mockResolvedValue(true);
    setIdentityCloudSaveAdapter({
      loadForIdentity: async () => null,
      saveForIdentity: async () => undefined,
      deleteForIdentity,
    });

    await expect(withIdentity(async () => deleteSaveGame(SLOT))).resolves.toBe(true);

    expect(deleteForIdentity).toHaveBeenCalledWith(SLOT, IDENTITY.playerId);
    await expect(fs.readFile(LOCAL_IDENTITY_PATH, "utf-8")).resolves.toContain('"sram"');
  });

  it("does not treat a local scoped file as an identity save during existence probes", async () => {
    await saveGame(createInitialGameState(), LOCAL_IDENTITY_SLOT);
    setIdentityCloudSaveAdapter({
      loadForIdentity: async () => null,
      saveForIdentity: async () => undefined,
      deleteForIdentity: async () => false,
    });

    await expect(withIdentity(async () => hasSaveGame(SLOT))).resolves.toBe(false);
  });
});
