/**
 * @jest-environment jsdom
 */

import { promises as fs } from "fs";
import {
  resetCloudSaveAdapters,
  setBrowserCloudSaveAdapter,
} from "@pokecrystal/core/adapters/cloud-save";
import { createInitialGameState } from "../state";
import { deleteSaveGame, hasSaveGame, loadGame, normalizeSaveSnapshot, saveGame } from "../save";
import { readGuestSessionSlot } from "../guest-session-storage";
import { createSerializedSnapshot } from "./save-test-harness";

const registerBrowserCloudSaveAdapter = (): void => {
  setBrowserCloudSaveAdapter({
    isConfigured: () => true,
    load: async (slot) => {
      const response = await fetch(`/api/savegame?slot=${encodeURIComponent(slot)}`, {
        cache: "no-store",
      });
      if (response.status === 401 || response.status === 404 || !response.ok) {
        return null;
      }
      const body = (await response.json().catch(() => null)) as {
        payload?: Record<string, unknown>;
        updated_at?: string | null;
        saved_at?: string | null;
      } | null;
      if (!body?.payload) {
        return null;
      }
      return {
        payload: body.payload,
        updated_at: body.updated_at ?? null,
        saved_at: body.saved_at ?? body.updated_at ?? null,
      };
    },
    save: async (slot, snapshot, savedAt) => {
      const response = await fetch("/api/savegame", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ slot, payload: snapshot, saved_at: savedAt }),
      });
      if (response.status === 401) {
        return false;
      }
      return response.ok;
    },
    delete: async (slot) => {
      const response = await fetch(`/api/savegame?slot=${encodeURIComponent(slot)}`, {
        method: "DELETE",
      });
      return response.ok;
    },
  });
};

describe("saveGame Supabase-first persistence", () => {
  const saveSlot = "test-supabase-priority.sav";
  const originalEnv = {
    url: process.env.NEXT_PUBLIC_SUPABASE_URL,
    anon: process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY,
  };

  beforeEach(() => {
    process.env.NEXT_PUBLIC_SUPABASE_URL = "https://example.supabase.co";
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY = "anon-key";
    registerBrowserCloudSaveAdapter();
  });

  afterEach(async () => {
    jest.useRealTimers();
    resetCloudSaveAdapters();
    process.env.NEXT_PUBLIC_SUPABASE_URL = originalEnv.url;
    process.env.NEXT_PUBLIC_SUPABASE_ANON_KEY = originalEnv.anon;
    jest.restoreAllMocks();
    await deleteSaveGame(saveSlot).catch(() => undefined);
    window.localStorage.clear();
    window.sessionStorage.clear();
  });

  it("uses the Supabase path without attempting a local mirror write", async () => {
    const fetchMock = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ ok: true, updated_at: "2026-03-30T12:00:00.000Z" }),
    });
    global.fetch = fetchMock as unknown as typeof fetch;
    const writeSpy = jest.spyOn(fs, "writeFile");

    await saveGame(createInitialGameState(), saveSlot);

    expect(fetchMock).toHaveBeenCalledWith(
      "/api/savegame",
      expect.objectContaining({ method: "POST" })
    );
    expect(writeSpy).not.toHaveBeenCalled();
  });

  it("falls back to guest storage when the Supabase save is unavailable", async () => {
    const fetchMock = jest.fn().mockResolvedValue({
      ok: false,
      status: 503,
      statusText: "Service Unavailable",
      json: async () => ({ error: "service unavailable" }),
    });
    global.fetch = fetchMock as unknown as typeof fetch;

    await expect(saveGame(createInitialGameState(), saveSlot)).resolves.toBe(true);
    expect(readGuestSessionSlot(saveSlot)).toEqual(expect.any(String));
  });

  it("ignores local filesystem write failures because no mirror write is attempted", async () => {
    const fetchMock = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ ok: true, updated_at: "2026-03-30T12:00:00.000Z" }),
    });
    global.fetch = fetchMock as unknown as typeof fetch;

    const quotaError = new Error("exceeded the quota");
    quotaError.name = "QuotaExceededError";
    jest.spyOn(fs, "writeFile").mockImplementation(() => Promise.reject(quotaError));

    await expect(saveGame(createInitialGameState(), saveSlot)).resolves.toBe(true);
    expect(fetchMock).toHaveBeenCalledWith(
      "/api/savegame",
      expect.objectContaining({ method: "POST" })
    );
  });

  it("loads the remote save and ignores local browser storage", async () => {
    const remoteState = createInitialGameState();
    remoteState.sram.player_name = "RemoteWins";
    const remoteSnapshot = normalizeSaveSnapshot(remoteState, "remote-test");

    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        ok: true,
        payload: remoteSnapshot,
        updated_at: "2026-03-30T12:00:00.000Z",
        saved_at: "2026-03-30T12:00:00.000Z",
      }),
    }) as unknown as typeof fetch;

    const loaded = await loadGame(saveSlot);

    expect(loaded.sram.player_name).toBe("RemoteWins");
  });

  it("loads the newer guest-session save when remote data is older", async () => {
    const guestState = createInitialGameState();
    guestState.sram.player_name = "GuestWins";
    global.fetch = jest.fn().mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ error: "Not signed in" }),
    }) as unknown as typeof fetch;

    await saveGame(guestState, saveSlot);

    const remoteState = createInitialGameState();
    remoteState.sram.player_name = "RemoteOlder";
    const remoteSnapshot = normalizeSaveSnapshot(remoteState, "remote-older");
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        ok: true,
        payload: remoteSnapshot,
        updated_at: "2026-03-30T12:00:00.000Z",
        saved_at: "2026-03-30T12:00:00.000Z",
      }),
    }) as unknown as typeof fetch;

    const loaded = await loadGame(saveSlot);

    expect(loaded.sram.player_name).toBe("GuestWins");
  });

  it("prefers the remote save when remote and guest-session timestamps are equal", async () => {
    jest.useFakeTimers().setSystemTime(new Date("2026-03-30T12:00:00.000Z"));
    const guestState = createInitialGameState();
    guestState.sram.player_name = "GuestSameTimestamp";
    global.fetch = jest.fn().mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ error: "Not signed in" }),
    }) as unknown as typeof fetch;

    await saveGame(guestState, saveSlot);

    const remoteSnapshot = createSerializedSnapshot("RemoteSameTimestamp", "remote-same-time");
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        ok: true,
        payload: remoteSnapshot,
        updated_at: "2026-03-30T12:00:00.000Z",
        saved_at: "2026-03-30T12:00:00.000Z",
      }),
    }) as unknown as typeof fetch;

    const loaded = await loadGame(saveSlot);

    expect(loaded.sram.player_name).toBe("RemoteSameTimestamp");
  });

  it("reports an existing save when only guest-session data is present", async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 401,
      json: async () => ({ error: "Not signed in" }),
    }) as unknown as typeof fetch;

    await saveGame(createInitialGameState(), saveSlot);

    await expect(hasSaveGame(saveSlot)).resolves.toBe(true);
  });

  it("reports an existing save when remote payload is invalid but guest-session data is readable", async () => {
    const guestState = createInitialGameState();
    guestState.sram.player_name = "GuestReadable";
    global.fetch = jest.fn().mockResolvedValueOnce({
      ok: false,
      status: 401,
      json: async () => ({ error: "Not signed in" }),
    }) as unknown as typeof fetch;

    await saveGame(guestState, saveSlot);

    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({
        ok: true,
        payload: { nope: "bad payload" },
        updated_at: "2026-03-30T12:00:00.000Z",
        saved_at: "2026-03-30T12:00:00.000Z",
      }),
    }) as unknown as typeof fetch;

    await expect(hasSaveGame(saveSlot)).resolves.toBe(true);
    await expect(loadGame(saveSlot)).resolves.toMatchObject({
      sram: expect.objectContaining({
        player_name: "GuestReadable",
      }),
    });
  });

  it("deletes guest-session data even when remote delete is unauthorized", async () => {
    const fetchMock = jest
      .fn()
      .mockResolvedValue({
        ok: false,
        status: 401,
        json: async () => ({ error: "Not signed in" }),
      })
      .mockResolvedValueOnce({
        ok: false,
        status: 401,
        json: async () => ({ error: "Not signed in" }),
      })
      .mockResolvedValueOnce({
        ok: false,
        status: 401,
        json: async () => ({ error: "Not signed in" }),
      });
    global.fetch = fetchMock as unknown as typeof fetch;

    await saveGame(createInitialGameState(), saveSlot);
    await expect(deleteSaveGame(saveSlot)).resolves.toBe(true);
    await expect(hasSaveGame(saveSlot)).resolves.toBe(false);
  });
});
